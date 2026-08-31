pub mod freelancer;
pub mod fiverr;
pub mod indeed;
pub mod upwork;

use crate::db::Db;
use crate::hunt;
use crate::models::{NewLead, ScrapeResponse};
use tokio::time::{sleep, Duration};

pub const USER_AGENTS: [&str; 4] = [
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Safari/605.1.15",
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:125.0) Gecko/20100101 Firefox/125.0",
];

fn http_client() -> Result<reqwest::Client, reqwest::Error> {
    let rng = rand::random::<usize>() % USER_AGENTS.len();
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENTS[rng])
        .default_headers({
            let mut h = reqwest::header::HeaderMap::new();
            if let Ok(accept) = reqwest::header::HeaderValue::from_str(
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            ) {
                h.insert(reqwest::header::ACCEPT, accept);
            }
            if let Ok(auth) = reqwest::header::HeaderValue::from_str("no-referrer-when-downgrade") {
                h.insert(reqwest::header::REFERER, auth);
            }
            h
        })
        .timeout(Duration::from_secs(25))
        .build()?;
    Ok(client)
}

fn dedupe_truncate(list: Vec<String>, max: usize) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    list.into_iter()
        .filter(|k| {
            let k = k.trim().to_lowercase();
            seen.insert(k)
        })
        .take(max)
        .collect()
}

pub async fn run_scrape(
    db: Db,
    sources: &Vec<String>,
    keywords: &Vec<String>,
    max_per_run: usize,
) -> ScrapeResponse {
    let keywords = dedupe_truncate(keywords.clone(), 20);
    // Indeed uses a plain `q` keyword query plus an optional `l` location.
    let location = db.get_setting("location");
    // Profile fit drives the auto-queue: any freshly discovered lead that scores
    // at/above the threshold is marked `queued` for the Discover page.
    let profile = hunt::Profile::from_db(&db);
    let queue_threshold = db.auto_queue_threshold();
    let mut inserted: i64 = 0;
    let mut total_found: i64 = 0;
    let mut errors: Vec<String> = Vec::new();

    for source in sources {
        let source = source.trim().to_lowercase();
        if source.is_empty() {
            continue;
        }
        for kw in &keywords {
            // polite rate limiting between requests
            sleep(Duration::from_millis(rand::random::<u64>() % 1500 + 1000)).await;
            let result: Result<Vec<NewLead>, String> = match source.as_str() {
                "upwork" | "upwork.com" => upwork::fetch(&kw).await,
                "freelancer" | "freelancer.com" => freelancer::fetch(&kw).await,
                "fiverr" | "fiverr.com" => fiverr::fetch(&kw).await,
                "indeed" | "indeed.com" => indeed::fetch(&kw, location.as_deref()).await,
                other => Err(format!("unknown source: {}", other)),
            };
            match result {
                Ok(leads) => {
                    let found = leads.len() as i64;
                    total_found += found;
                    for lead in leads.into_iter().take(max_per_run) {
                        match db.insert_lead(&lead) {
                            Ok(true) => {
                                inserted += 1;
                                queue_if_fit(&db, &lead, &profile, queue_threshold);
                            }
                            Ok(false) => {}
                            Err(e) => errors.push(format!("db error: {}", e)),
                        }
                    }
                }
                Err(e) => errors.push(format!("[{} / {}] {}", source, kw, e)),
            }
        }
    }

    ScrapeResponse {
        inserted,
        total_found,
        errors,
    }
}

/// Score a freshly inserted lead against the user profile and, if it meets the
/// fit threshold, mark it `queued` so the Discover page surfaces it for a
/// one-click tailored application.
fn queue_if_fit(db: &Db, lead: &NewLead, profile: &hunt::Profile, threshold: i64) {
    let Ok(Some(id)) = db.lead_id_by_url(&lead.url) else {
        return;
    };
    let Ok(Some(full)) = db.get_lead(id) else {
        return;
    };
    let s = hunt::score_lead_against_profile(0, &full, profile);
    if s != full.score {
        let _ = db.update_lead_score(id, s);
    }
    if s >= threshold {
        let _ = db.set_lead_queued(id, true);
    }
}
