mod api;
mod db;
mod models;
mod scraper;

use axum::http::{header, Method};
use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "leadgen=info,axum=info".into()),
        )
        .init();

    let db_path = std::env::var("DATABASE_PATH").unwrap_or_else(|_| "leadgen.db".into());
    let bind = std::env::var("BIND").unwrap_or_else(|_| "0.0.0.0:8080".into());

    let db = db::Db::new(&db_path).expect("failed to open database");

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::PATCH, Method::DELETE, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);

    let app = api::router(db)
        .layer(cors)
        .layer(tower_http::trace::TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(&bind).await.unwrap();
    tracing::info!("leadgen backend listening on {}", bind);
    axum::serve(listener, app).await.unwrap();
}