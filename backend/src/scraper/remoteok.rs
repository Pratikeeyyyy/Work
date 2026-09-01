use crate::models::NewLead;
use crate::scraper::http_client;
use crate::scraper::{matches_keyword, strip_html};
use serde::Deserialize;
use std::sync::Mutex;
use std::time::{Duration, Instant};

// RemoteOK: legal public JSON API (https://remoteok.com/api) listing remote tech
// jobs. No auth is required. The API returns the whole (large) recent set, so we
// fetch it once, cache it briefly, and filter per keyword client-side.

const API_URL: &str = "https://remoteok.com/api";
const CACHE_TTL: Duration = Duration::from_secs(300);

struct Cache {
    fetched_at: Option<Instant>,
    items: Vec<NewLead>,
}

static CACHE: Mutex<Option<Cache>> = Mutex::new(None);

#[derive(Deserialize)]
#[serde(untagged)]
enum Item {
    Job(RawJob),
    #[allow(dead_code)]
    Meta(serde_json::Value),
}
#[derive(Deserialize)]
struct RawJob {
    #[serde(default)]
    position: String,
    #[serde(default)]
    company: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    location: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    apply_url: String,
    #[serde(default)]
    date: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    salary_min: i64,
    #[serde(default)]
    salary_max: i64,
}

pub async fn fetch(keyword: &str) -> Result<Vec<NewLead>, String> {
    let fresh = {
        let guard = CACHE
            .lock()
            .map_err(|_| "cache lock poisoned".to_string())?;
        match guard.as_ref() {
            Some(c) if c
                .fetched_at
                .map(|t| t.elapsed() < CACHE_TTL)
                .unwrap_or(false) =>
            {
                true
            }
            _ => false,
        }
    };

    if !fresh {
        let items = fetch_feed().await?;
        let mut guard = CACHE
            .lock()
            .map_err(|_| "cache lock poisoned".to_string())?;
        *guard = Some(Cache {
            fetched_at: Some(Instant::now()),
            items,
        });
    }

    let guard = CACHE
        .lock()
        .map_err(|_| "cache lock poisoned".to_string())?;
    let cached = guard.as_ref().expect("cache filled above");
    Ok(cached
        .items
        .iter()
        .filter(|lead| matches_keyword(lead, keyword))
        .cloned()
        .collect())
}

async fn fetch_feed() -> Result<Vec<NewLead>, String> {
    let client = http_client().map_err(|e| format!("http client: {}", e))?;
    let resp = client
        .get(API_URL)
        .send()
        .await
        .map_err(|e| format!("request failed: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("status {}", resp.status()));
    }
    let body = resp.text().await.map_err(|e| format!("read body: {}", e))?;
    parse(&body)
}

fn parse(body: &str) -> Result<Vec<NewLead>, String> {
    let items: Vec<Item> = serde_json::from_str(body).map_err(|e| format!("json: {}", e))?;
    let mut out = Vec::new();
    for item in items {
        let j = match item {
            Item::Job(j) => j,
            Item::Meta(_) => continue,
        };
        let title = j.position.trim().to_string();
        if title.is_empty() {
            continue;
        }
        // Prefer the apply_url; fall back to the listing url.
        let url = {
            let u = if !j.apply_url.trim().is_empty() {
                j.apply_url.trim().to_string()
            } else {
                j.url.trim().to_string()
            };
            if u.is_empty() {
                continue;
            }
            u
        };
        let budget = if j.salary_min > 0 || j.salary_max > 0 {
            let lo = if j.salary_min > 0 { j.salary_min.to_string() } else { "?".into() };
            let hi = if j.salary_max > 0 { j.salary_max.to_string() } else { "?".into() };
            Some(format!("${}-${}", lo, hi))
        } else {
            None
        };
        let company = j.company.trim().to_string();
        out.push(NewLead {
            source: "remoteok".into(),
            title,
            description: if j.description.trim().is_empty() {
                format!("Sourced from RemoteOK. Apply at: {}", url)
            } else {
                strip_html(&j.description)
            },
            url,
            budget,
            budget_min: if j.salary_min > 0 { Some(j.salary_min as f64) } else { None },
            budget_max: if j.salary_max > 0 { Some(j.salary_max as f64) } else { None },
            currency: Some("USD".to_string()),
            location: {
                let l = j.location.trim().to_string();
                if l.is_empty() {
                    Some("Remote".to_string())
                } else {
                    Some(l)
                }
            },
            technologies: {
                let t = j.tags.join(", ");
                if t.trim().is_empty() {
                    None
                } else {
                    Some(t)
                }
            },
            client_name: if company.is_empty() {
                None
            } else {
                Some(company)
            },
            posted_date: {
                let d = j.date.trim().to_string();
                if d.is_empty() {
                    None
                } else {
                    Some(d)
                }
            },
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_skips_meta() {
        let body = r#"[
          {"not_a_job": true},
          {"id":"1","position":"React Engineer","company":"Acme","url":"https://remoteok.com/j1","apply_url":"https://apply","tags":["react"],"date":"2026-08-30T00:00:00+00:00","description":"<p>React &amp; Rust</p>","location":"Worldwide","salary_min":90000,"salary_max":120000}
        ]"#;
        let leads = parse(body).unwrap();
        assert_eq!(leads.len(), 1);
        assert_eq!(leads[0].source, "remoteok");
        assert_eq!(leads[0].url, "https://apply");
        assert_eq!(leads[0].client_name.as_deref(), Some("Acme"));
        assert_eq!(leads[0].budget.as_deref(), Some("$90000-$120000"));
        assert_eq!(leads[0].currency.as_deref(), Some("USD"));
    }

    #[test]
    fn empty_position_skipped() {
        let body = r#"[{"url":"https://x","position":""}]"#;
        assert!(parse(body).unwrap().is_empty());
    }
}
