use crate::db::Db;
use crate::models::*;
use crate::scraper;
use axum::extract::{Path, Query, State};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use serde::Deserialize;

pub fn router(db: Db) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/stats", get(stats))
        .route("/leads", get(list_leads).post(add_lead))
        .route("/leads/:id", get(get_lead).delete(remove_lead))
        .route("/leads/:id/status", patch(update_status))
        .route("/leads/:id/notes", patch(update_notes))
        .route("/leads/:id/to-client", post(to_client))
        .route("/clients", get(list_clients).post(add_client))
        .route("/clients/:id", get(get_client).put(update_client).delete(remove_client))
        .route("/contracts", get(list_contracts).post(add_contract))
        .route("/contracts/:id/deploy", post(deploy_contract))
        .route("/contracts/:id/status", patch(update_contract_status))
        .route("/settings/keywords", get(get_keywords).put(put_keywords))
        .route("/settings/sources", get(get_sources).put(put_sources))
        .route("/scrape", post(scrape))
        .with_state(db)
}

async fn health() -> &'static str {
    "ok"
}

async fn stats(State(db): State<Db>) -> Json<Stats> {
    Json(db.stats().unwrap_or_else(|_e| {
        Stats {
            total_leads: 0,
            new_leads: 0,
            applied_leads: 0,
            won_leads: 0,
            total_clients: 0,
            active_clients: 0,
            total_contracts: 0,
            by_source: vec![],
            top_technologies: vec![],
        }
    }))
}

#[derive(Deserialize)]
pub struct LeadQuery {
    pub source: Option<String>,
    pub status: Option<String>,
    pub q: Option<String>,
    pub limit: Option<i64>,
}

async fn list_leads(
    State(db): State<Db>,
    Query(q): Query<LeadQuery>,
) -> Result<Json<Vec<Lead>>, (axum::http::StatusCode, String)> {
    let limit = q.limit.unwrap_or(200).clamp(1, 1000);
    db.list_leads(
        q.source.as_deref(),
        q.status.as_deref(),
        q.q.as_deref(),
        limit,
    )
    .map(Json)
    .map_err(rusqlite_err)
}

async fn get_lead(
    State(db): State<Db>,
    Path(id): Path<i64>,
) -> Result<Json<Lead>, (axum::http::StatusCode, String)> {
    db.get_lead(id)
        .map_err(rusqlite_err)?
        .map(Json)
        .ok_or((axum::http::StatusCode::NOT_FOUND, "lead not found".into()))
}

async fn add_lead(
    State(db): State<Db>,
    Json(mut l): Json<NewLead>,
) -> Result<Json<ApiMessage>, (axum::http::StatusCode, String)> {
    if l.source.trim().is_empty() {
        l.source = "manual".into();
    }
    if l.title.trim().is_empty() {
        return Err((axum::http::StatusCode::BAD_REQUEST, "title required".into()));
    }
    if l.url.trim().is_empty() {
        l.url = format!("manual://{}", rand::random::<u64>());
    }
    db.insert_lead(&l).map_err(rusqlite_err)?;
    Ok(Json(ApiMessage {
        message: "lead added".into(),
    }))
}

async fn remove_lead(
    State(db): State<Db>,
    Path(id): Path<i64>,
) -> Result<Json<ApiMessage>, (axum::http::StatusCode, String)> {
    db.delete_lead(id).map_err(rusqlite_err)?;
    Ok(Json(ApiMessage {
        message: "lead deleted".into(),
    }))
}

async fn update_status(
    State(db): State<Db>,
    Path(id): Path<i64>,
    Json(s): Json<StatusUpdate>,
) -> Result<Json<ApiMessage>, (axum::http::StatusCode, String)> {
    let valid = ["new", "shortlisted", "applied", "responded", "won", "lost", "archived"];
    if !valid.contains(&s.status.as_str()) {
        return Err((axum::http::StatusCode::BAD_REQUEST, "invalid status".into()));
    }
    if db.get_lead(id).map_err(rusqlite_err)?.is_none() {
        return Err((axum::http::StatusCode::NOT_FOUND, "lead not found".into()));
    }
    db.update_lead_status(id, &s.status).map_err(rusqlite_err)?;
    Ok(Json(ApiMessage {
        message: "status updated".into(),
    }))
}

async fn update_notes(
    State(db): State<Db>,
    Path(id): Path<i64>,
    Json(n): Json<serde_json::Value>,
) -> Result<Json<ApiMessage>, (axum::http::StatusCode, String)> {
    let notes = n.get("notes").and_then(|v| v.as_str()).unwrap_or("");
    db.update_lead_notes(id, notes).map_err(rusqlite_err)?;
    Ok(Json(ApiMessage {
        message: "notes updated".into(),
    }))
}

async fn to_client(
    State(db): State<Db>,
    Path(id): Path<i64>,
) -> Result<Json<ApiMessage>, (axum::http::StatusCode, String)> {
    if db.get_lead(id).map_err(rusqlite_err)?.is_none() {
        return Err((axum::http::StatusCode::NOT_FOUND, "lead not found".into()));
    }
    db.client_from_lead(id).map_err(rusqlite_err)?;
    db.update_lead_status(id, "applied").map_err(rusqlite_err)?;
    Ok(Json(ApiMessage {
        message: "converted to client".into(),
    }))
}

async fn list_clients(
    State(db): State<Db>,
    Query(q): Query<LeadQuery>,
) -> Result<Json<Vec<Client>>, (axum::http::StatusCode, String)> {
    db.list_clients(q.status.as_deref())
        .map(Json)
        .map_err(rusqlite_err)
}

async fn get_client(
    State(db): State<Db>,
    Path(id): Path<i64>,
) -> Result<Json<Client>, (axum::http::StatusCode, String)> {
    db.get_client(id)
        .map_err(rusqlite_err)?
        .map(Json)
        .ok_or((axum::http::StatusCode::NOT_FOUND, "client not found".into()))
}

async fn add_client(
    State(db): State<Db>,
    Json(c): Json<NewClient>,
) -> Result<Json<ApiMessage>, (axum::http::StatusCode, String)> {
    if c.name.trim().is_empty() {
        return Err((axum::http::StatusCode::BAD_REQUEST, "name required".into()));
    }
    db.insert_client(&c).map_err(rusqlite_err)?;
    Ok(Json(ApiMessage {
        message: "client added".into(),
    }))
}

async fn update_client(
    State(db): State<Db>,
    Path(id): Path<i64>,
    Json(mut c): Json<Client>,
) -> Result<Json<ApiMessage>, (axum::http::StatusCode, String)> {
    c.id = id;
    db.update_client(&c).map_err(rusqlite_err)?;
    Ok(Json(ApiMessage {
        message: "client updated".into(),
    }))
}

async fn remove_client(
    State(db): State<Db>,
    Path(id): Path<i64>,
) -> Result<Json<ApiMessage>, (axum::http::StatusCode, String)> {
    db.delete_client(id).map_err(rusqlite_err)?;
    Ok(Json(ApiMessage {
        message: "client deleted".into(),
    }))
}

async fn list_contracts(
    State(db): State<Db>,
) -> Result<Json<Vec<Contract>>, (axum::http::StatusCode, String)> {
    db.list_contracts()
        .map(Json)
        .map_err(rusqlite_err)
}

async fn add_contract(
    State(db): State<Db>,
    Json(c): Json<NewContract>,
) -> Result<Json<ApiMessage>, (axum::http::StatusCode, String)> {
    if db.get_client(c.client_id).map_err(rusqlite_err)?.is_none() {
        return Err((axum::http::StatusCode::BAD_REQUEST, "unknown client".into()));
    }
    db.insert_contract(&c).map_err(rusqlite_err)?;
    Ok(Json(ApiMessage {
        message: "contract created (status: draft). Deploy the escrow contract on-chain when both sides agree.".into(),
    }))
}

#[derive(Deserialize)]
pub struct DeployRequest {
    pub tx_hash: Option<String>,
    pub contract_address: Option<String>,
}

async fn deploy_contract(
    State(db): State<Db>,
    Path(id): Path<i64>,
    body: Option<Json<DeployRequest>>,
) -> Result<Json<ApiMessage>, (axum::http::StatusCode, String)> {
    // When the frontend deploys contracts/FreelanceEscrow.sol from a wallet it
    // sends back the real tx hash + deployed address. Without a body we fall back
    // to a stub hash so the flow is still demoable offline.
    let contracts = db.list_contracts().map_err(rusqlite_err)?;
    if !contracts.iter().any(|c| c.id == id) {
        return Err((axum::http::StatusCode::NOT_FOUND, "contract not found".into()));
    }
    let tx_hash = body
        .as_ref()
        .and_then(|b| b.tx_hash.clone())
        .filter(|h| !h.trim().is_empty())
        .unwrap_or_else(|| format!("0x{:x}", rand::random::<u64>().max(1)));
    let contract_address = body.as_ref().and_then(|b| b.contract_address.clone());
    db.update_contract_deployment(id, "deployed", &tx_hash, contract_address.as_deref())
        .map_err(rusqlite_err)?;
    Ok(Json(ApiMessage {
        message: format!("Escrow deployed. tx hash: {tx_hash}."),
    }))
}

async fn update_contract_status(
    State(db): State<Db>,
    Path(id): Path<i64>,
    Json(s): Json<StatusUpdate>,
) -> Result<Json<ApiMessage>, (axum::http::StatusCode, String)> {
    let valid = [
        "deployed",
        "funded",
        "in_progress",
        "submitted",
        "completed",
        "disputed",
        "refunded",
    ];
    if !valid.contains(&s.status.as_str()) {
        return Err((axum::http::StatusCode::BAD_REQUEST, "invalid status".into()));
    }
    let contracts = db.list_contracts().map_err(rusqlite_err)?;
    if !contracts.iter().any(|c| c.id == id) {
        return Err((axum::http::StatusCode::NOT_FOUND, "contract not found".into()));
    }
    db.update_contract_status(id, &s.status).map_err(rusqlite_err)?;
    Ok(Json(ApiMessage {
        message: "contract status updated".into(),
    }))
}

async fn get_keywords(State(db): State<Db>) -> Json<KeywordSetting> {
    Json(KeywordSetting {
        keywords: db.get_keywords(),
    })
}

async fn put_keywords(
    State(db): State<Db>,
    Json(k): Json<KeywordSetting>,
) -> Result<Json<ApiMessage>, (axum::http::StatusCode, String)> {
    let cleaned: Vec<String> = k
        .keywords
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    db.set_setting("keywords", &cleaned.join(", "))
        .map_err(rusqlite_err)?;
    Ok(Json(ApiMessage {
        message: "keywords saved".into(),
    }))
}

async fn get_sources(State(db): State<Db>) -> Json<KeywordSetting> {
    let raw = db.get_setting("sources").unwrap_or_else(|| "upwork.freelancer.fiverr".into());
    Json(KeywordSetting {
        keywords: raw.split(',').map(|s| s.trim().to_string()).collect(),
    })
}

async fn put_sources(
    State(db): State<Db>,
    Json(k): Json<KeywordSetting>,
) -> Result<Json<ApiMessage>, (axum::http::StatusCode, String)> {
    let cleaned: Vec<String> = k
        .keywords
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    db.set_setting("sources", &cleaned.join(","))
        .map_err(rusqlite_err)?;
    Ok(Json(ApiMessage {
        message: "sources saved".into(),
    }))
}

async fn scrape(
    State(db): State<Db>,
    Json(req): Json<ScrapeRequest>,
) -> Result<Json<ScrapeResponse>, (axum::http::StatusCode, String)> {
    let max_per_run: usize = db
        .get_setting("max_leads_per_run")
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);
    let sources = match req.sources {
        Some(s) if !s.is_empty() => s,
        _ => db
            .get_setting("sources")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
    };
    let keywords = match req.keywords {
        Some(k) if !k.is_empty() => k,
        _ => db.get_keywords(),
    };
    if keywords.is_empty() {
        return Err((axum::http::StatusCode::BAD_REQUEST, "no keywords configured".into()));
    }
    let result = scraper::run_scrape(db, &sources, &keywords, max_per_run).await;
    Ok(Json(result))
}

fn rusqlite_err(e: rusqlite::Error) -> (axum::http::StatusCode, String) {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        format!("database error: {}", e),
    )
}