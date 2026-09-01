use crate::models::NewLead;
use crate::scraper::http_client;
use crate::scraper::{matches_keyword, strip_html};
use rss::Channel;
use std::sync::Mutex;
use std::time::{Duration, Instant};

// We Work Remotely: legal public RSS feed of remote jobs (no auth).
// https://weworkremotely.com/categories/remote-programming-jobs.rss
// RSS supports keyword filtering client-side, so we cache the feed briefly so a
// multi-keyword run doesn't re-hit the server for every keyword.

const FEED_URL: &str = "https://weworkremotely.com/categories/remote-programming-jobs.rss";
const CACHE_TTL: Duration = Duration::from_secs(300);

struct Cache {
    fetched_at: Option<Instant>,
    items: Vec<NewLead>,
}

static CACHE: Mutex<Option<Cache>> = Mutex::new(None);

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
        .get(FEED_URL)
        .send()
        .await
        .map_err(|e| format!("request failed: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("status {}", resp.status()));
    }
    let body = resp.text().await.map_err(|e| format!("read body: {}", e))?;
    parse(&body, FEED_URL)
}

fn parse(body: &str, _fallback_base: &str) -> Result<Vec<NewLead>, String> {
    let channel = Channel::read_from(body.as_bytes())
        .map_err(|e| format!("rss parse: {}", e))?;
    let mut out = Vec::new();
    for item in channel.items() {
        let title = item.title().unwrap_or("").trim().to_string();
        if title.is_empty() {
            continue;
        }
        let link = item.link().unwrap_or("").trim().to_string();
        if link.is_empty() {
            continue;
        }
        let pub_date = {
            let d = item.pub_date().unwrap_or("").to_string();
            if d.is_empty() {
                None
            } else {
                Some(d)
            }
        };
        out.push(NewLead {
            source: "weworkremotely".into(),
            title: {
                // keep the role portion as the title when the "Company: Role"
                // convention is used, else whole title
                client_name(&title)
                    .map(|(_, role)| role)
                    .unwrap_or(title.clone())
            },
            description: {
                let d = item.description().map(|x| strip_html(x)).unwrap_or_default();
                if d.is_empty() {
                    format!("Sourced from We Work Remotely. Apply at: {}", link)
                } else {
                    d
                }
            },
            url: link,
            budget: None,
            budget_min: None,
            budget_max: None,
            currency: None,
            location: Some("Remote / Worldwide".to_string()),
            technologies: None,
            client_name: client_name(&title).map(|(c, _)| c),
            posted_date: pub_date,
        });
    }
    Ok(out)
}

/// Extract "<Company>: <role>" style titles into (company, role). Returns None
/// when there is no ":" separator.
fn client_name(title: &str) -> Option<(String, String)> {
    let idx = title.find(':')?;
    let comp = title[..idx].trim().to_string();
    let role = title[idx + 1..].trim().to_string();
    if comp.is_empty() || role.is_empty() {
        None
    } else {
        Some((comp, role))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_company_role() {
        let t = "Acme Inc: Senior Rust Engineer";
        let (c, r) = client_name(t).unwrap();
        assert_eq!(c, "Acme Inc");
        assert_eq!(r, "Senior Rust Engineer");
        assert!(client_name("no separator here").is_none());
    }

    #[test]
    fn parses_rss() {
        let body = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0"><channel><title>jobs</title>
<item>
<title>Acme Corp: React Developer</title>
<link>https://weworkremotely.com/remote-jobs/react</link>
<pubDate>Wed, 12 Aug 2026 18:38:32 +0000</pubDate>
<description><![CDATA[<p>Some <b>HTML</b> &amp; text</p>]]></description>
</item>
</channel></rss>"#;
        let leads = parse(body, "").unwrap();
        assert_eq!(leads.len(), 1);
        assert_eq!(leads[0].source, "weworkremotely");
        assert_eq!(leads[0].title, "React Developer");
        assert_eq!(leads[0].client_name.as_deref(), Some("Acme Corp"));
        assert!(leads[0].description.contains("HTML"));
        assert!(!leads[0].description.contains('<'));
        assert_eq!(
            leads[0].url,
            "https://weworkremotely.com/remote-jobs/react"
        );
    }

    #[test]
    fn keyword_filters() {
        let lead = NewLead {
            source: "weworkremotely".into(),
            title: "React Developer".into(),
            description: String::new(),
            url: "https://x".into(),
            budget: None,
            budget_min: None,
            budget_max: None,
            currency: None,
            location: None,
            technologies: Some("typescript".into()),
            client_name: None,
            posted_date: None,
        };
        assert!(matches_keyword(&lead, "react"));
        assert!(matches_keyword(&lead, "React TypeScript"));
        assert!(!matches_keyword(&lead, "golang"));
    }
}
