mod api;
mod auth;
mod db;
mod hunt;
mod models;
mod scraper;

use axum::http::{header, HeaderValue, Method};
use tower_http::cors::{Any, CorsLayer};

/// Background auto-discovery loop. For every registered account, polls the
/// reliable remote-job feeds (Remotive, We Work Remotely, RemoteOK — all legal
/// public APIs) on that user's configured interval, scoring and auto-queueing
/// high-fit leads into the user's own isolated data database. Disabled unless
/// `auto_pull_enabled` is set (default off, so dev/staging stays deterministic
/// and polite).
async fn auto_discovery_loop(users_db: db::Db) {
    use std::time::Duration;
    tracing::info!("auto-discovery worker started");
    loop {
        for username in users_db.list_users().unwrap_or_default() {
            let user_db = db::user_db_for(&username);
            let enabled = user_db.auto_pull_enabled();
            let interval = user_db.auto_pull_interval_mins().max(10);
            let keywords = user_db.get_keywords();
            if enabled && !keywords.is_empty() {
                tracing::info!(
                    "auto-discovery pull starting for {username} (every {}m)",
                    interval
                );
                let result = scraper::run_scrape(
                    user_db.clone(),
                    &vec![
                        "remotive".to_string(),
                        "weworkremotely".to_string(),
                        "remoteok".to_string(),
                    ],
                    &keywords,
                    100,
                )
                .await;
                user_db.set_last_auto_pull(&chrono::Utc::now().naive_utc().to_string());
                tracing::info!(
                    "auto-discovery done for {username}: inserted={} errors={}",
                    result.inserted,
                    result.errors.len()
                );
            }
        }
        tokio::time::sleep(Duration::from_secs(600)).await;
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "leadgen=info,axum=info".into()),
        )
        .init();

    let db_path = std::env::var("DATABASE_PATH").unwrap_or_else(|_| "leadgen.db".into());

    // Per-user data files live alongside the account registry database so they
    // are covered by the same volume/persistence.
    let user_dir = std::path::Path::new(&db_path)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| ".".to_string());
    db::set_user_data_dir(&user_dir);

    // Render injects PORT; local dev can set BIND or fall back to 8080.
    let bind = std::env::var("BIND").unwrap_or_else(|_| {
        std::env::var("PORT")
            .map(|p| format!("0.0.0.0:{p}"))
            .unwrap_or_else(|_| "0.0.0.0:8080".into())
    });

    let db = db::Db::new(&db_path).expect("failed to open database");

    // Kick off the scheduled auto-discovery worker with a cloned handle.
    tokio::spawn(auto_discovery_loop(db.clone()));

    // In production, restrict CORS to the frontend origin(s) via CORS_ORIGIN
    // (comma-separated). Defaults to allowing any origin for local dev.
    let cors = match std::env::var("CORS_ORIGIN") {
        Ok(value) if !value.trim().is_empty() => {
            let origins: Vec<HeaderValue> = value
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .filter_map(|s| HeaderValue::from_str(&s).ok())
                .collect();
            CorsLayer::new()
                .allow_origin(origins)
                .allow_methods([Method::GET, Method::POST, Method::PUT, Method::PATCH, Method::DELETE, Method::OPTIONS])
                .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
        }
        _ => CorsLayer::new()
            .allow_origin(Any)
            .allow_methods([Method::GET, Method::POST, Method::PUT, Method::PATCH, Method::DELETE, Method::OPTIONS])
            .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]),
    };

    let app = api::router(db)
        .layer(cors)
        .layer(tower_http::trace::TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(&bind).await.unwrap();
    tracing::info!("leadgen backend listening on {}", bind);
    axum::serve(listener, app).await.unwrap();
}