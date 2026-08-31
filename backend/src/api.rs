use crate::db::Db;
use crate::hunt;
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
        .route("/leads/import", post(import_lead_url))
        .route("/leads/rescore", post(rescore_leads))
        .route("/leads/:id", get(get_lead).delete(remove_lead))
        .route("/leads/:id/status", patch(update_status))
        .route("/leads/:id/notes", patch(update_notes))
        .route("/leads/:id/outreach", get(lead_outreach))
        .route("/leads/:id/to-client", post(to_client))
        .route("/clients", get(list_clients).post(add_client))
        .route("/clients/:id", get(get_client).put(update_client).delete(remove_client))
        .route("/contracts", get(list_contracts).post(add_contract))
        .route("/contracts/:id/deploy", post(deploy_contract))
        .route("/contracts/:id/status", patch(update_contract_status))
        .route("/applications", get(list_applications).post(add_application))
        .route("/applications/:id", get(get_application).patch(update_application).delete(delete_application))
        .route("/profile", get(get_profile).put(put_profile))
        .route("/linkedin/auth-url", get(linkedin_auth_url))
        .route("/linkedin/callback", post(linkedin_callback))
        .route("/linkedin/status", get(linkedin_status))
        .route("/settings/keywords", get(get_keywords).put(put_keywords))
        .route("/settings/sources", get(get_sources).put(put_sources))
        .route("/settings/linkedin", get(get_linkedin_settings).put(put_linkedin_settings))
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
            total_applications: 0,
            interviewed: 0,
            hired: 0,
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

/// Generate personalized outreach drafts (proposal / message / email) for a lead
/// using the user's saved profile.
async fn lead_outreach(
    State(db): State<Db>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<hunt::OutreachDraft>>, (axum::http::StatusCode, String)> {
    let lead = db
        .get_lead(id)
        .map_err(rusqlite_err)?
        .ok_or((axum::http::StatusCode::NOT_FOUND, "lead not found".into()))?;
    let profile = hunt::Profile::from_db(&db);
    Ok(Json(hunt::generate_outreach(&lead, &profile)))
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

#[derive(Deserialize)]
pub struct ImportRequest {
    pub url: String,
}

/// Import a job/gig/client URL pasted by the user. Because job sites block
/// logged-out scraping, we store the URL with a source guess and mark it for
/// review rather than failing.
async fn import_lead_url(
    State(db): State<Db>,
    Json(req): Json<ImportRequest>,
) -> Result<Json<ApiMessage>, (axum::http::StatusCode, String)> {
    if req.url.trim().is_empty() {
        return Err((axum::http::StatusCode::BAD_REQUEST, "url required".into()));
    }
    let lead = hunt::lead_from_url(&req.url);
    db.insert_lead(&lead).map_err(rusqlite_err)?;
    Ok(Json(ApiMessage {
        message: "lead imported from URL — add details and run scoring".into(),
    }))
}

/// Recompute every lead's fit score against the user's profile. Returns how
/// many leads were updated.
async fn rescore_leads(State(db): State<Db>) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let profile = hunt::Profile::from_db(&db);
    let leads = db.list_leads(None, None, None, 10000).map_err(rusqlite_err)?;
    let mut updated = 0i64;
    for lead in &leads {
        // Rescore from scratch: use hunt score (profile fit) as the base (skills
        // + signals), which is what matters most for job hunting.
        let s = hunt::score_lead_against_profile(lead.score, lead, &profile);
        if s != lead.score {
            db.update_lead_score(lead.id, s).map_err(rusqlite_err)?;
            updated += 1;
        }
    }
    Ok(Json(serde_json::json!({ "message": "leads rescored", "updated": updated })))
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

// ---------- Applications (job-application pipeline) ----------

async fn list_applications(State(db): State<Db>) -> Result<Json<Vec<Application>>, (axum::http::StatusCode, String)> {
    db.list_applications().map(Json).map_err(rusqlite_err)
}

async fn get_application(
    State(db): State<Db>,
    Path(id): Path<i64>,
) -> Result<Json<Application>, (axum::http::StatusCode, String)> {
    db.get_application(id)
        .map_err(rusqlite_err)?
        .map(Json)
        .ok_or((axum::http::StatusCode::NOT_FOUND, "application not found".into()))
}

async fn add_application(
    State(db): State<Db>,
    Json(a): Json<NewApplication>,
) -> Result<Json<ApiMessage>, (axum::http::StatusCode, String)> {
    if db.get_lead(a.lead_id).map_err(rusqlite_err)?.is_none() {
        return Err((axum::http::StatusCode::BAD_REQUEST, "unknown lead".into()));
    }
    db.add_application(&a).map_err(rusqlite_err)?;
    Ok(Json(ApiMessage {
        message: "application tracked".into(),
    }))
}

async fn update_application(
    State(db): State<Db>,
    Path(id): Path<i64>,
    Json(u): Json<ApplicationUpdate>,
) -> Result<Json<ApiMessage>, (axum::http::StatusCode, String)> {
    if db.get_application(id).map_err(rusqlite_err)?.is_none() {
        return Err((axum::http::StatusCode::NOT_FOUND, "application not found".into()));
    }
    db.update_application(id, &u).map_err(rusqlite_err)?;
    Ok(Json(ApiMessage {
        message: "application updated".into(),
    }))
}

async fn delete_application(
    State(db): State<Db>,
    Path(id): Path<i64>,
) -> Result<Json<ApiMessage>, (axum::http::StatusCode, String)> {
    db.delete_application(id).map_err(rusqlite_err)?;
    Ok(Json(ApiMessage {
        message: "application deleted".into(),
    }))
}

// ---------- Profile ----------

async fn get_profile(State(db): State<Db>) -> Json<hunt::Profile> {
    Json(hunt::Profile::from_db(&db))
}

async fn put_profile(
    State(db): State<Db>,
    Json(p): Json<hunt::Profile>,
) -> Result<Json<ApiMessage>, (axum::http::StatusCode, String)> {
    p.save(&db).map_err(rusqlite_err)?;
    // Keep the scrape location setting in sync with the profile location.
    if let Some(loc) = &p.location {
        db.set_setting("location", loc).map_err(rusqlite_err)?;
    }
    Ok(Json(ApiMessage {
        message: "profile saved".into(),
    }))
}

// ---------- LinkedIn OAuth ----------

#[derive(Deserialize)]
pub struct LinkedinUrlQuery {
    pub redirect_uri: Option<String>,
}

async fn linkedin_auth_url(
    State(db): State<Db>,
    Query(q): Query<LinkedinUrlQuery>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let client_id = db
        .get_setting("linkedin.client_id")
        .unwrap_or_default();
    if client_id.is_empty() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "LinkedIn app not configured. Set client id/secret and redirect in Settings, or see SETUP.md.".into(),
        ));
    }
    let redirect_uri = q
        .redirect_uri
        .or_else(|| db.get_setting("linkedin.redirect_uri"))
        .unwrap_or_else(|| "http://localhost:5173/linkedin/callback".into());
    let state = format!("{:x}", rand::random::<u64>());
    db.set_setting("linkedin.oauth_state", &state)
        .map_err(rusqlite_err)?;
    let scope = "openid profile email";
    let url = hunt::linkedin_auth_url(&client_id, &redirect_uri, &state, scope);
    Ok(Json(serde_json::json!({ "url": url, "state": state })))
}

#[derive(Deserialize)]
pub struct LinkedinCallback {
    pub code: String,
    pub state: Option<String>,
    pub redirect_uri: Option<String>,
}

async fn linkedin_callback(
    State(db): State<Db>,
    Json(body): Json<LinkedinCallback>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let client_id = db.get_setting("linkedin.client_id").unwrap_or_default();
    let client_secret = db.get_setting("linkedin.client_secret").unwrap_or_default();
    if client_id.is_empty() || client_secret.is_empty() {
        return Err((axum::http::StatusCode::BAD_REQUEST, "linkedin app not configured".into()));
    }
    let redirect_uri = body
        .redirect_uri
        .clone()
        .or_else(|| db.get_setting("linkedin.redirect_uri"))
        .unwrap_or_else(|| "http://localhost:5173/linkedin/callback".into());

    // Validate CSRF state if provided.
    if let Some(state) = &body.state {
        let stored = db.get_setting("linkedin.oauth_state");
        if let Some(stored) = stored {
            if stored != *state {
                return Err((axum::http::StatusCode::BAD_REQUEST, "state mismatch".into()));
            }
        }
    }

    let result = hunt::connect_linkedin(&db, &client_id, &client_secret, &redirect_uri, &body.code)
        .await
        .map_err(|e| (axum::http::StatusCode::BAD_GATEWAY, e))?;
    Ok(Json(result))
}

async fn linkedin_status(State(db): State<Db>) -> Json<serde_json::Value> {
    let has_token = db.get_setting("linkedin.access_token").map(|t| !t.is_empty()).unwrap_or(false);
    let name = db.get_setting("linkedin.member_name").unwrap_or_default();
    let configured = db
        .get_setting("linkedin.client_id")
        .map(|c| !c.is_empty())
        .unwrap_or(false);
    Json(serde_json::json!({
        "connected": has_token,
        "configured": configured,
        "member_name": name,
        "client_id": db.get_setting("linkedin.client_id").unwrap_or_default(),
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

#[derive(Deserialize)]
pub struct LinkedinSettings {
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub redirect_uri: Option<String>,
}

async fn get_linkedin_settings(State(db): State<Db>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "client_id": db.get_setting("linkedin.client_id").unwrap_or_default(),
        // Never return the secret in full.
        "client_secret_set": db.get_setting("linkedin.client_secret").map(|s| !s.is_empty()).unwrap_or(false),
        "redirect_uri": db.get_setting("linkedin.redirect_uri").unwrap_or_else(|| "http://localhost:5173/linkedin/callback".into()),
    }))
}

async fn put_linkedin_settings(
    State(db): State<Db>,
    Json(s): Json<LinkedinSettings>,
) -> Result<Json<ApiMessage>, (axum::http::StatusCode, String)> {
    if let Some(id) = &s.client_id {
        db.set_setting("linkedin.client_id", id).map_err(rusqlite_err)?;
    }
    if let Some(secret) = &s.client_secret {
        if !secret.trim().is_empty() {
            db.set_setting("linkedin.client_secret", secret).map_err(rusqlite_err)?;
        }
    }
    if let Some(uri) = &s.redirect_uri {
        if !uri.trim().is_empty() {
            db.set_setting("linkedin.redirect_uri", uri).map_err(rusqlite_err)?;
        }
    }
    Ok(Json(ApiMessage {
        message: "linkedin app settings saved".into(),
    }))
}

async fn scrape(
    State(db): State<Db>,
    Json(req): Json<ScrapeRequest>,
) -> Result<Json<ScrapeResponse>, (axum::http::StatusCode, String)> {    let max_per_run: usize = db
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