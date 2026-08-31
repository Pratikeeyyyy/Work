use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lead {
    pub id: i64,
    pub source: String,
    pub title: String,
    pub description: String,
    pub url: String,
    pub budget: Option<String>,
    pub budget_min: Option<f64>,
    pub budget_max: Option<f64>,
    pub currency: Option<String>,
    pub location: Option<String>,
    pub technologies: Option<String>,
    pub client_name: Option<String>,
    pub posted_date: Option<String>,
    pub status: String,
    pub score: i64,
    pub notes: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewLead {
    pub source: String,
    pub title: String,
    pub description: String,
    pub url: String,
    pub budget: Option<String>,
    pub budget_min: Option<f64>,
    pub budget_max: Option<f64>,
    pub currency: Option<String>,
    pub location: Option<String>,
    pub technologies: Option<String>,
    pub client_name: Option<String>,
    pub posted_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Client {
    pub id: i64,
    pub lead_id: Option<i64>,
    pub name: String,
    pub email: Option<String>,
    pub company: Option<String>,
    pub country: Option<String>,
    pub website: Option<String>,
    pub whatsapp: Option<String>,
    pub source: Option<String>,
    pub linkedin: Option<String>,
    pub past_work: Option<String>,
    pub preferences: Option<String>,
    pub status: String,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewClient {
    pub lead_id: Option<i64>,
    pub name: String,
    pub email: Option<String>,
    pub company: Option<String>,
    pub country: Option<String>,
    pub website: Option<String>,
    pub whatsapp: Option<String>,
    pub source: Option<String>,
    pub linkedin: Option<String>,
    pub past_work: Option<String>,
    pub preferences: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contract {
    pub id: i64,
    pub client_id: i64,
    pub client_address: Option<String>,
    pub freelancer_address: Option<String>,
    pub contract_address: Option<String>,
    pub title: String,
    pub amount_wei: Option<String>,
    pub currency: String,
    pub status: String,
    pub tx_hash: Option<String>,
    pub deployed_at: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewContract {
    pub client_id: i64,
    pub client_address: Option<String>,
    pub freelancer_address: Option<String>,
    pub contract_address: Option<String>,
    pub title: String,
    pub amount_wei: Option<String>,
    pub currency: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    pub total_leads: i64,
    pub new_leads: i64,
    pub applied_leads: i64,
    pub won_leads: i64,
    pub total_clients: i64,
    pub active_clients: i64,
    pub total_contracts: i64,
    pub total_applications: i64,
    pub interviewed: i64,
    pub hired: i64,
    pub by_source: Vec<SourceCount>,
    pub top_technologies: Vec<TechCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceCount {
    pub source: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechCount {
    pub tech: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapeRequest {
    pub sources: Option<Vec<String>>,
    pub keywords: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapeResponse {
    pub inserted: i64,
    pub total_found: i64,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusUpdate {
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiMessage {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordSetting {
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Application {
    pub id: i64,
    pub lead_id: i64,
    pub client_id: Option<i64>,
    pub status: String,
    pub applied_at: Option<String>,
    pub replied_at: Option<String>,
    pub interviewed_at: Option<String>,
    pub offered_at: Option<String>,
    pub hired_at: Option<String>,
    pub company: Option<String>,
    pub contact: Option<String>,
    pub next_scheduled: Option<String>,
    pub follow_up_count: i64,
    pub last_follow_up: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub lead_title: Option<String>,
    pub lead_url: Option<String>,
    pub lead_source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewApplication {
    pub lead_id: i64,
    pub client_id: Option<i64>,
    pub company: Option<String>,
    pub contact: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationUpdate {
    pub status: Option<String>,
    pub applied_at: Option<String>,
    pub replied_at: Option<String>,
    pub interviewed_at: Option<String>,
    pub offered_at: Option<String>,
    pub hired_at: Option<String>,
    pub company: Option<String>,
    pub contact: Option<String>,
    pub next_scheduled: Option<String>,
    pub notes: Option<String>,
    pub follow_up: bool,
}