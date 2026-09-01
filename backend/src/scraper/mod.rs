pub mod freelancer;
pub mod fiverr;
pub mod indeed;
pub mod remotive;
pub mod remoteok;
pub mod upwork;
pub mod weworkremotely;

use crate::db::Db;
use crate::hunt;
use crate::models::{NewLead, ScrapeResponse};
use futures::future::join_all;
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

    // Fetch each enabled source concurrently (the per-keyword politeness sleep
    // lives inside each source's own loop). DB writes happen only after all
    // fetches complete, so the shared connection is never held across an await.
    let batches = sources
        .iter()
        .filter_map(|s| {
            let source = s.trim().to_lowercase();
            if source.is_empty() {
                return None;
            }
            Some(fetch_source(
                source,
                keywords.clone(),
                location.clone(),
                max_per_run,
            ))
        });

    // Bound the whole run so a blocked source can never hang the request.
    let batches = match tokio::time::timeout(Duration::from_secs(150), join_all(batches)).await {
        Ok(batches) => batches,
        Err(_) => {
            return ScrapeResponse {
                inserted: 0,
                total_found: 0,
                errors: vec![
                    "scrape stopped after 150s; reduce the sources in Settings for a faster run"
                        .into(),
                ],
            };
        }
    };

    let mut inserted: i64 = 0;
    let mut total_found: i64 = 0;
    let mut errors: Vec<String> = Vec::new();
    for (found, leads, mut source_errors) in batches {
        total_found += found;
        errors.append(&mut source_errors);
        for lead in leads {
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

    // Re-sync the high-fit queue against the current profile/threshold so leads
    // discovered earlier (or profile/threshold changes) are surfaced correctly.
    db.recompute_queued(&profile, queue_threshold);

    ScrapeResponse {
        inserted,
        total_found,
        errors,
    }
}

/// Crawl one source over every keyword. Returns found leads, a count, and any
/// per-source errors so a single bad source can't abort the whole run.
async fn fetch_source(
    source: String,
    keywords: Vec<String>,
    location: Option<String>,
    max_per_run: usize,
) -> (i64, Vec<NewLead>, Vec<String>) {
    let mut leads: Vec<NewLead> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    for kw in &keywords {
        // polite rate limiting between requests to the same source
        sleep(Duration::from_millis(rand::random::<u64>() % 1400 + 1000)).await;
        let result: Result<Vec<NewLead>, String> = match source.as_str() {
            "upwork" | "upwork.com" => upwork::fetch(kw).await,
            "freelancer" | "freelancer.com" => freelancer::fetch(kw).await,
            "fiverr" | "fiverr.com" => fiverr::fetch(kw).await,
            "indeed" | "indeed.com" => indeed::fetch(kw, location.as_deref()).await,
            "remotive" | "remotive.com" => remotive::fetch(kw).await,
            "weworkremotely" | "weworkremotely.com" => weworkremotely::fetch(kw).await,
            "remoteok" | "remoteok.com" => remoteok::fetch(kw).await,
            other => Err(format!("unknown source: {}", other)),
        };
        match result {
            Ok(found) => leads.extend(found.into_iter().take(max_per_run)),
            Err(e) => errors.push(format!("[{} / {}] {}", source, kw, e)),
        }
    }
    let found = leads.len() as i64;
    (found, leads, errors)
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

/// Remove HTML tags and decode common entities so scraped descriptions are
/// readable. Shared by the HTML/JSON sources.
pub(crate) fn strip_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_tag = false;
    let mut tag_buf = String::new();
    for ch in input.chars() {
        if ch == '<' {
            in_tag = true;
            tag_buf.clear();
        } else if ch == '>' {
            in_tag = false;
            out.push(' ');
        } else if in_tag {
            tag_buf.push(ch);
        } else {
            out.push(ch);
        }
    }
    out.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Case-insensitive keyword relevance check used to filter full feeds (WWR,
/// RemoteOK) down to items matching the user's keyword. Checks title +
/// description + tags, allowing plural/partial matches.
pub(crate) fn matches_keyword(lead: &NewLead, keyword: &str) -> bool {
    let kw = keyword.trim().to_lowercase();
    if kw.is_empty() {
        return true;
    }
    let haystack = format!(
        "{} {} {}",
        lead.title,
        lead.description,
        lead.technologies.as_deref().unwrap_or("")
    )
    .to_lowercase();
    kw.split_whitespace().all(|tok| {
        let tok = tok.trim_matches(|c: char| !c.is_alphanumeric());
        tok.is_empty() || haystack.contains(tok)
    })
}
