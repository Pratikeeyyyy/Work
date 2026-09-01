use crate::auth;
use crate::db::Db;
use crate::hunt;
use crate::models::*;
use crate::scraper;
use axum::extract::{Extension, Path, Query, Request, State};
use axum::http::StatusCode;
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use serde::Deserialize;

/// Build the application router. `db` is the central account registry (users
/// table). Each authenticated request is routed to its own isolated per-user
/// data database via the auth middleware, which injects that user's `Db` as a
/// request `Extension` for the protected handlers.
pub fn router(db: Db) -> Router {
    // Public routes: health check, account registration, login, status, logout.
    let public = Router::new()
        .route("/health", get(health))
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/status", get(auth_status));

    // Everything else requires a valid bearer token.
    let protected = Router::new()
        .route("/stats", get(stats))
        .route("/leads", get(list_leads).post(add_lead))
        .route("/leads/import", post(import_lead_url))
        .route("/leads/rescore", post(rescore_leads))
        .route("/leads/queue", get(lead_queue))
        .route("/leads/:id", get(get_lead).delete(remove_lead))
        .route("/leads/:id/status", patch(update_status))
        .route("/leads/:id/notes", patch(update_notes))
        .route("/leads/:id/outreach", get(lead_outreach))
        .route("/leads/:id/apply", get(lead_apply_kit))
        .route("/leads/:id/to-client", post(to_client))
        .route("/clients", get(list_clients).post(add_client))
        .route("/clients/:id", get(get_client).put(update_client).delete(remove_client))
        .route("/contracts", get(list_contracts).post(add_contract))
        .route("/contracts/:id/deploy", post(deploy_contract))
        .route("/contracts/:id/status", patch(update_contract_status))
        .route("/applications", get(list_applications).post(add_application))
        .route("/applications/due", get(applications_due))
        .route("/applications/:id", get(get_application).patch(update_application).delete(delete_application))
        .route("/profile", get(get_profile).put(put_profile))
        .route("/linkedin/auth-url", get(linkedin_auth_url))
        .route("/linkedin/callback", post(linkedin_callback))
        .route("/linkedin/status", get(linkedin_status))
        .route("/settings/keywords", get(get_keywords).put(put_keywords))
        .route("/settings/sources", get(get_sources).put(put_sources))
        .route("/settings/linkedin", get(get_linkedin_settings).put(put_linkedin_settings))
        .route("/settings/auto-update", get(get_auto_update_settings).put(put_auto_update_settings))
        .route("/scrape", post(scrape))
        .layer(middleware::from_fn(auth_middleware));

    public.merge(protected).with_state(db)
}

/// Reject requests that do not carry a valid bearer token. Preflight (CORS)
/// and the public routes are let through. For valid tokens, resolve the
/// authenticated user's username and inject their isolated data `Db` into the
/// request so downstream handlers operate only on that user's data.
async fn auth_middleware(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    if method == axum::http::Method::OPTIONS {
        return next.run(req).await;
    }
    let username = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .and_then(auth::username_for_token);
    match username {
        Some(username) => {
            let mut req = req;
            req.extensions_mut().insert(crate::db::user_db_for(&username));
            next.run(req).await
        }
        None => (StatusCode::UNAUTHORIZED, "unauthorized".to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

/// Validate a username (trimmed) for account registration. Returns an error
/// message, or `None` when the username is acceptable.
fn username_error(username: &str) -> Option<&'static str> {
    let name = username.trim();
    if name.len() < 3 || name.len() > 64 {
        return Some("username must be 3-64 characters");
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
    {
        return Some("username may only contain letters, digits, _ - .");
    }
    None
}

/// Validate a password for account registration.
fn password_error(password: &str) -> Option<&'static str> {
    if password.len() < 8 {
        return Some("password must be at least 8 characters");
    }
    if password.len() > 128 {
        return Some("password must be at most 128 characters");
    }
    None
}

/// Create a new account and return an authenticated session. The user's data is
/// stored in an isolated database created here, so each account is fully
/// independent (no shared leads/profile/settings).
async fn register(
    State(db): State<Db>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if let Some(msg) = username_error(&req.username) {
        return Err((StatusCode::BAD_REQUEST, msg.into()));
    }
    if let Some(msg) = password_error(&req.password) {
        return Err((StatusCode::BAD_REQUEST, msg.into()));
    }
    let username = req.username.trim().to_string();
    if db.user_exists(&username) {
        return Err((StatusCode::CONFLICT, "username already taken".into()));
    }
    let hash = auth::hash_password(&req.password);
    db.register_user(&username, &hash)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("failed to register: {e}")))?;
    // Create this user's isolated data database (full schema + default settings).
    db.create_user_data(&username)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "failed to initialize account data".into()))?;
    let token = auth::create_session(&username);
    Ok(Json(serde_json::json!({
        "message": "account created",
        "token": token,
        "username": username,
    })))
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

async fn login(
    State(db): State<Db>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if req.username.trim().is_empty() || req.password.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "username and password are required".into()));
    }
    let username = req.username.trim();
    let stored = db.user_password_hash(username);
    match stored {
        Some(hash) if auth::verify_password(&req.password, &hash) => {
            let token = auth::create_session(username);
            Ok(Json(serde_json::json!({ "token": token, "username": username })))
        }
        _ => Err((StatusCode::UNAUTHORIZED, "invalid username or password".into())),
    }
}

async fn auth_status(
    req: Request,
) -> Json<serde_json::Value> {
    let authenticated = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .and_then(auth::username_for_token);
    Json(serde_json::json!({
        "authenticated": authenticated.is_some(),
        "username": authenticated,
    }))
}

async fn logout(req: Request) -> Response {
    if let Some(tok) = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
    {
        auth::revoke_session(tok);
    }
    (StatusCode::OK, "logged out").into_response()
}

async fn health() -> &'static str {
    "ok"
}

async fn stats(Extension(db): Extension<Db>) -> Json<Stats> {
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
    Extension(db): Extension<Db>,
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
    Extension(db): Extension<Db>,
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
    Extension(db): Extension<Db>,
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
    Extension(db): Extension<Db>,
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
    Extension(db): Extension<Db>,
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
async fn rescore_leads(Extension(db): Extension<Db>) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
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
    Extension(db): Extension<Db>,
    Path(id): Path<i64>,
) -> Result<Json<ApiMessage>, (axum::http::StatusCode, String)> {
    db.delete_lead(id).map_err(rusqlite_err)?;
    Ok(Json(ApiMessage {
        message: "lead deleted".into(),
    }))
}

async fn update_status(
    Extension(db): Extension<Db>,
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
    Extension(db): Extension<Db>,
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
    Extension(db): Extension<Db>,
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
    Extension(db): Extension<Db>,
    Query(q): Query<LeadQuery>,
) -> Result<Json<Vec<Client>>, (axum::http::StatusCode, String)> {
    db.list_clients(q.status.as_deref())
        .map(Json)
        .map_err(rusqlite_err)
}

async fn get_client(
    Extension(db): Extension<Db>,
    Path(id): Path<i64>,
) -> Result<Json<Client>, (axum::http::StatusCode, String)> {
    db.get_client(id)
        .map_err(rusqlite_err)?
        .map(Json)
        .ok_or((axum::http::StatusCode::NOT_FOUND, "client not found".into()))
}

async fn add_client(
    Extension(db): Extension<Db>,
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
    Extension(db): Extension<Db>,
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
    Extension(db): Extension<Db>,
    Path(id): Path<i64>,
) -> Result<Json<ApiMessage>, (axum::http::StatusCode, String)> {
    db.delete_client(id).map_err(rusqlite_err)?;
    Ok(Json(ApiMessage {
        message: "client deleted".into(),
    }))
}

async fn list_contracts(
    Extension(db): Extension<Db>,
) -> Result<Json<Vec<Contract>>, (axum::http::StatusCode, String)> {
    db.list_contracts()
        .map(Json)
        .map_err(rusqlite_err)
}

async fn add_contract(
    Extension(db): Extension<Db>,
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
    Extension(db): Extension<Db>,
    Path(id): Path<i64>,
    Json(body): Json<DeployRequest>,
) -> Result<Json<ApiMessage>, (axum::http::StatusCode, String)> {
    // A real on-chain deployment is required: both the transaction hash and the
    // deployed contract address must be supplied by the wallet that sends the
    // deployment transaction. There is intentionally no demo/stub fallback.
    let tx_hash = body
        .tx_hash
        .clone()
        .unwrap_or_default()
        .trim()
        .to_string();
    let contract_address = body
        .contract_address
        .clone()
        .unwrap_or_default()
        .trim()
        .to_string();
    if tx_hash.len() < 2 || !tx_hash.starts_with("0x") {
        return Err((axum::http::StatusCode::BAD_REQUEST, "a valid tx_hash (0x…) is required".into()));
    }
    if contract_address.len() < 2 || !contract_address.starts_with("0x") {
        return Err((axum::http::StatusCode::BAD_REQUEST, "a contract_address (0x…) is required".into()));
    }
    let contracts = db.list_contracts().map_err(rusqlite_err)?;
    if !contracts.iter().any(|c| c.id == id) {
        return Err((axum::http::StatusCode::NOT_FOUND, "contract not found".into()));
    }
    db.update_contract_deployment(id, "deployed", &tx_hash, Some(&contract_address))
        .map_err(rusqlite_err)?;
    Ok(Json(ApiMessage {
        message: "Escrow deployment recorded.".into(),
    }))
}

async fn update_contract_status(
    Extension(db): Extension<Db>,
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

async fn list_applications(Extension(db): Extension<Db>) -> Result<Json<Vec<Application>>, (axum::http::StatusCode, String)> {
    db.list_applications().map(Json).map_err(rusqlite_err)
}

async fn get_application(
    Extension(db): Extension<Db>,
    Path(id): Path<i64>,
) -> Result<Json<Application>, (axum::http::StatusCode, String)> {
    db.get_application(id)
        .map_err(rusqlite_err)?
        .map(Json)
        .ok_or((axum::http::StatusCode::NOT_FOUND, "application not found".into()))
}

async fn add_application(
    Extension(db): Extension<Db>,
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
    Extension(db): Extension<Db>,
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
    Extension(db): Extension<Db>,
    Path(id): Path<i64>,
) -> Result<Json<ApiMessage>, (axum::http::StatusCode, String)> {
    db.delete_application(id).map_err(rusqlite_err)?;
    Ok(Json(ApiMessage {
        message: "application deleted".into(),
    }))
}

// ---------- Profile ----------

async fn get_profile(Extension(db): Extension<Db>) -> Json<hunt::Profile> {
    Json(hunt::Profile::from_db(&db))
}

async fn put_profile(
    Extension(db): Extension<Db>,
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
    Extension(db): Extension<Db>,
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
    Extension(db): Extension<Db>,
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

async fn linkedin_status(Extension(db): Extension<Db>) -> Json<serde_json::Value> {
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


async fn get_keywords(Extension(db): Extension<Db>) -> Json<KeywordSetting> {
    Json(KeywordSetting {
        keywords: db.get_keywords(),
    })
}

async fn put_keywords(
    Extension(db): Extension<Db>,
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

async fn get_sources(Extension(db): Extension<Db>) -> Json<KeywordSetting> {
    let raw = db.get_setting("sources").unwrap_or_else(|| "upwork.freelancer.fiverr".into());
    Json(KeywordSetting {
        keywords: raw.split(',').map(|s| s.trim().to_string()).collect(),
    })
}

async fn put_sources(
    Extension(db): Extension<Db>,
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

async fn get_linkedin_settings(Extension(db): Extension<Db>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "client_id": db.get_setting("linkedin.client_id").unwrap_or_default(),
        // Never return the secret in full.
        "client_secret_set": db.get_setting("linkedin.client_secret").map(|s| !s.is_empty()).unwrap_or(false),
        "redirect_uri": db.get_setting("linkedin.redirect_uri").unwrap_or_else(|| "http://localhost:5173/linkedin/callback".into()),
    }))
}

async fn put_linkedin_settings(
    Extension(db): Extension<Db>,
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
    Extension(db): Extension<Db>,
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

#[derive(Deserialize)]
struct AutoUpdateSettings {
    enabled: Option<bool>,
    interval_mins: Option<u64>,
    threshold: Option<i64>,
}

/// High-fit auto-queue: freshly discovered leads that matched your profile and
/// were auto-added for a quick, tailored application.
async fn lead_queue(
    Extension(db): Extension<Db>,
) -> Result<Json<Vec<Lead>>, (axum::http::StatusCode, String)> {
    db.list_queued_leads().map(Json).map_err(rusqlite_err)
}

/// One-click tailored application kit (legal *review-and-confirm*). Returns the
/// real source URL to open plus the outreach copy pre-drafted from your profile.
/// You review and submit on the source site — nothing is auto-submitted.
async fn lead_apply_kit(
    Extension(db): Extension<Db>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let lead = db
        .get_lead(id)
        .map_err(rusqlite_err)?
        .ok_or((axum::http::StatusCode::NOT_FOUND, "lead not found".into()))?;
    let profile = hunt::Profile::from_db(&db);
    let outreach = hunt::generate_outreach(&lead, &profile);
    Ok(Json(serde_json::json!({
        "lead": lead,
        "apply_url": lead.url,
        "source": lead.source,
        "outreach": outreach,
        "contact": {
            "name": profile.name,
            "email": profile.email,
            "portfolio": profile.portfolio,
            "github": profile.github,
            "linkedin": profile.linkedin,
        },
    })))
}

/// Applications in the live pipeline that need a nudge right now, so you know
/// exactly who to follow up with today.
async fn applications_due(
    Extension(db): Extension<Db>,
) -> Result<Json<Vec<Application>>, (axum::http::StatusCode, String)> {
    db.list_applications_due().map(Json).map_err(rusqlite_err)
}

async fn get_auto_update_settings(Extension(db): Extension<Db>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "enabled": db.auto_pull_enabled(),
        "interval_mins": db.auto_pull_interval_mins(),
        "threshold": db.auto_queue_threshold(),
        "last_pull": db.get_last_auto_pull(),
    }))
}

async fn put_auto_update_settings(
    Extension(db): Extension<Db>,
    Json(s): Json<AutoUpdateSettings>,
) -> Result<Json<ApiMessage>, (axum::http::StatusCode, String)> {
    if let Some(enabled) = s.enabled {
        db.set_setting("auto_pull_enabled", if enabled { "1" } else { "0" })
            .map_err(rusqlite_err)?;
    }
    if let Some(mins) = s.interval_mins {
        if mins < 10 {
            return Err((axum::http::StatusCode::BAD_REQUEST, "interval must be at least 10 minutes".into()));
        }
        db.set_setting("auto_pull_interval_mins", &mins.to_string())
            .map_err(rusqlite_err)?;
    }
    if let Some(threshold) = s.threshold {
        db.set_setting("auto_queue_threshold", &threshold.to_string())
            .map_err(rusqlite_err)?;
    }
    Ok(Json(ApiMessage {
        message: "auto-discovery settings saved".into(),
    }))
}

fn rusqlite_err(e: rusqlite::Error) -> (axum::http::StatusCode, String) {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        format!("database error: {}", e),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn username_validation_accepts_good_names() {
        assert_eq!(username_error("alice"), None);
        assert_eq!(username_error("alice123"), None);
        assert_eq!(username_error("a_b-c.d"), None);
        assert_eq!(username_error("  alice  "), None);
    }

    #[test]
    fn username_validation_rejects_bad_names() {
        assert!(username_error("").is_some());
        assert!(username_error("ab").is_some());
        assert!(username_error(&"a".repeat(65)).is_some());
        assert!(username_error("al!ce").is_some());
        assert!(username_error("al ice").is_some());
        assert!(username_error("🚀").is_some());
    }

    #[test]
    fn password_validation_checks_length() {
        assert!(!password_error("password").is_some());
        assert!(password_error("short").is_some());
        assert!(password_error(&"a".repeat(129)).is_some());
    }
}