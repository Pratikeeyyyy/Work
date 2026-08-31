use crate::models::NewLead;
use crate::scraper::http_client;
use urlencoding::encode;

// Upwork historically published public RSS job feeds, but discontinued them in
// August 2024 (the old /ab/feed/jobs/rss URLs now return errors or login walls).
// We try the RSS feed first (harmless if it still works) and fall back to
// scanning the public search page when it does not. Both surfaces are actively
// bot-protected, so failures surface as source errors rather than crashing.
pub async fn fetch(keyword: &str) -> Result<Vec<NewLead>, String> {
    let client = http_client().map_err(|e| format!("http client: {}", e))?;

    // 1) RSS feed (legacy; often blocked now).
    let rss_url = format!(
        "https://www.upwork.com/ab/feed/jobs/rss?keywords={}&sort=recency&job_type=billing&api_full_job_description=1",
        encode(keyword)
    );
    if let Ok(resp) = client.get(&rss_url).send().await {
        if resp.status().is_success() {
            if let Ok(body) = resp.text().await {
                if let Ok(leads) = parse_rss(&body) {
                    if !leads.is_empty() {
                        return Ok(leads);
                    }
                }
            }
        }
    }

    // 2) Public search page fallback.
    let page_url = format!(
        "https://www.upwork.com/nx/search/jobs/?q={}&sort=recency",
        encode(keyword)
    );
    let resp = client
        .get(&page_url)
        .header("X-Requested-With", "XMLHttpRequest")
        .send()
        .await
        .map_err(|e| format!("request failed: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!(
            "Upwork blocked this request (status {}). Add manual leads with the job URLs you find.",
            resp.status()
        ));
    }
    let body = resp.text().await.map_err(|e| format!("read body: {}", e))?;
    parse_search_page(&body)
}

fn parse_rss(body: &str) -> Result<Vec<NewLead>, String> {
    let feed = rss::Channel::read_from(body.as_bytes()).map_err(|e| format!("rss parse: {}", e))?;
    let mut leads = Vec::new();
    for item in feed.items() {
        let title = item.title().unwrap_or("").trim().to_string();
        if title.is_empty() {
            continue;
        }
        let link = item.link().unwrap_or("").to_string();
        let description = strip_html(item.description().unwrap_or(""));
        let pub_date = item.pub_date().map(|s| s.to_string());
        let (budget, location) = extract_meta(&title, &description);
        // tags from the RSS category elements (may include technologies)
        let mut tags: Vec<String> = Vec::new();
        if !item.categories().is_empty() {
            for c in item.categories() {
                tags.push(c.name.trim().to_string());
            }
        }
        let key_matches: Vec<String> = tags
            .iter()
            .filter(|t| keyword_relevant(t))
            .cloned()
            .collect();
        let technologie_tags = if key_matches.is_empty() {
            None
        } else {
            Some(key_matches.join(", "))
        };

        leads.push(NewLead {
            source: "upwork".into(),
            title,
            description: truncate(&description, 4000),
            url: link,
            budget,
            budget_min: None,
            budget_max: None,
            currency: Some("USD".into()),
            location,
            technologies: technologie_tags,
            client_name: None,
            posted_date: pub_date,
        });
    }
    Ok(leads)
}

/// Parse Upwork's search page for job postings. The page embeds structured JSON
/// (e.g. `window.__JOB_POSTINGS_LIST_DATA__`) plus human-readable anchor text.
/// We defensively scan for known markers and fall back to generic anchor/link
/// extraction so it keeps working across layout changes.
fn parse_search_page(body: &str) -> Result<Vec<NewLead>, String> {
    let mut leads = Vec::new();

    // Try the embedded JSON blob(s) first.
    if let Some(leads_from_json) = parse_embedded_json(body) {
        if !leads_from_json.is_empty() {
            return Ok(leads_from_json);
        }
    }

    // Fall back to scanning anchors that look like job postings.
    let href_re = regex::Regex::new(r#"href="(/jobs/[^"]+|/freelance-jobs/[^"]+|/nx/search/jobs/[^"]+)"#).unwrap();
    let text_re =
        regex::Regex::new(r#"class="[^"]*job[^"]*"[^>]*>\s*([A-Za-z0-9][^<]{20,200})"#).unwrap();
    let mut hrefs: Vec<String> = href_re
        .captures_iter(body)
        .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
        .collect();
    hrefs.dedup();
    let titles: Vec<String> = text_re
        .captures_iter(body)
        .filter_map(|c| c.get(1).map(|m| m.as_str().trim().to_string()))
        .collect();

    for (i, href) in hrefs.into_iter().take(25).enumerate() {
        let url = if href.starts_with("http") {
            href.clone()
        } else {
            format!("https://www.upwork.com{}", href)
        };
        let title = titles
            .get(i)
            .cloned()
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| "Upwork job posting".to_string());
        let tech = keyword_clean(&title);
        leads.push(NewLead {
            source: "upwork".into(),
            title,
            description: "Job found on Upwork — open the link to see full details.".to_string(),
            url,
            budget: None,
            budget_min: None,
            budget_max: None,
            currency: Some("USD".into()),
            location: None,
            technologies: Some(tech),
            client_name: None,
            posted_date: None,
        });
    }
    Ok(leads)
}

/// Parse a `window.__JOB_POSTINGS_LIST_DATA__ = {...}` blob embedded in the page.
fn parse_embedded_json(body: &str) -> Option<Vec<NewLead>> {
    let marker = "__JOB_POSTINGS_LIST_DATA__";
    let idx = body.find(marker)?;
    let start = body[idx..].find('=')? + idx + 1;
    let rest = &body[start..];
    let json_start = rest.find('{')? + start;
    let slice = &body[json_start..];
    let end = slice.find("</script>")?;
    let raw = &slice[..end];
    let raw = raw.trim_end();
    let close = raw.rfind('}')?;
    let json_str = &raw[..=close];

    let data: serde_json::Value = serde_json::from_str(json_str).ok()?;
    let mut leads = Vec::new();
    // Defensive traversal for an array of job objects.
    fn walk(node: &serde_json::Value, title_key: &[&str], out: &mut Vec<NewLead>, depth: usize) {
        if depth > 8 || out.len() > 100 {
            return;
        }
        match node {
            serde_json::Value::Array(arr) => {
                let looks_like_job = arr.iter().take(1).all(|v| {
                    v.is_object()
                        && (v.get("op_title").is_some()
                            || v.get("title").is_some()
                            || v.get("jobTitle").is_some())
                });
                if looks_like_job {
                    for job in arr {
                        if let Some(l) = job_to_lead(job, title_key) {
                            out.push(l);
                        }
                    }
                } else {
                    for item in arr {
                        walk(item, title_key, out, depth + 1);
                    }
                }
            }
            serde_json::Value::Object(map) => {
                for v in map.values() {
                    walk(v, title_key, out, depth + 1);
                }
            }
            _ => {}
        }
    }
    walk(&data, &["op_title", "title", "jobTitle"], &mut leads, 0);
    if leads.is_empty() {
        None
    } else {
        Some(leads)
    }
}

fn job_to_lead(job: &serde_json::Value, title_key: &[&str]) -> Option<NewLead> {
    let title = title_key
        .iter()
        .find_map(|k| job.get(*k).and_then(|v| v.as_str()))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())?;

    let url = job
        .get("jobUri")
        .or_else(|| job.get("url"))
        .or_else(|| job.get("link"))
        .and_then(|v| v.as_str())
        .map(|s| {
            if s.starts_with("http") {
                s.to_string()
            } else if s.starts_with('/') {
                format!("https://www.upwork.com{}", s)
            } else {
                s.to_string()
            }
        })
        .unwrap_or_else(|| "https://www.upwork.com/".to_string());

    let (budget, budget_min, budget_max) = extract_budget_fields(job);

    let technologies = job
        .get("skills")
        .and_then(|s| s.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|s| s.as_str().or_else(|| s.get("name").and_then(|n| n.as_str())))
                .take(6)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|s| !s.is_empty())
        .or_else(|| Some(keyword_clean(&title)));

    Some(NewLead {
        source: "upwork".into(),
        title,
        description: job
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| truncate(&strip_html(s), 4000))
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "Job found on Upwork — open the link to see full details.".to_string()),
        url,
        budget,
        budget_min,
        budget_max,
        currency: Some("USD".into()),
        location: job
            .get("country")
            .or_else(|| job.get("location"))
            .and_then(|v| v.as_str())
            .map(String::from),
        technologies,
        client_name: None,
        posted_date: None,
    })
}

fn extract_budget_fields(job: &serde_json::Value) -> (Option<String>, Option<f64>, Option<f64>) {
    // Upwork jobs expose either a fixed budget or an hourly rate range.
    let fixed = job
        .get("budget")
        .or_else(|| job.get("price"))
        .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)));
    if let Some(f) = fixed.filter(|x| *x > 0.0) {
        return (Some(format!("${:.0} fixed", f)), Some(f), Some(f));
    }
    let min = job
        .get("rate")
        .or_else(|| job.get("hourlyRate"))
        .and_then(|r| {
            r.get("min").and_then(|v| v.as_f64()).or_else(|| {
                r.get("lower")
                    .and_then(|v| v.as_f64())
                    .or_else(|| r.as_f64())
            })
        });
    let max = job
        .get("rate")
        .or_else(|| job.get("hourlyRate"))
        .and_then(|r| {
            r.get("max").and_then(|v| v.as_f64()).or_else(|| {
                r.get("higher")
                    .and_then(|v| v.as_f64())
                    .or_else(|| r.as_f64())
            })
        });
    match (min, max) {
        (Some(a), Some(b)) => (
            Some(format!("${:.0}/hr - ${:.0}/hr", a, b)),
            Some(a),
            Some(b),
        ),
        (Some(a), None) => (Some(format!("${:.0}/hr+", a)), Some(a), None),
        _ => (None, None, None),
    }
}

fn strip_html(s: &str) -> String {
    let re = regex::Regex::new(r"<[^>]*>").unwrap();
    let cleaned = re
        .replace_all(s, " ")
        .replace("&amp;", "&")
        .replace("&#39;", "'")
        .replace("&quot;", "\"")
        .replace("&nbsp;", " ");
    let mut collapsed = String::new();
    let mut prev_space = false;
    for c in cleaned.chars() {
        if c.is_whitespace() {
            if !prev_space {
                collapsed.push(' ');
            }
            prev_space = true;
        } else {
            collapsed.push(c);
            prev_space = false;
        }
    }
    collapsed.trim().to_string()
}

fn extract_meta(title: &str, description: &str) -> (Option<String>, Option<String>) {
    let mut budget: Option<String> = None;
    let mut location: Option<String> = None;

    // Common Upwork title pattern: "Title (Budget: $500, Location: USA)"
    if let Some(start) = title.find('(') {
        if let Some(end) = title[start..].find(')') {
            let meta = &title[start + 1..start + end];
            for part in meta.split(',') {
                let p = part.trim();
                let lower = p.to_lowercase();
                if lower.starts_with("budget:") {
                    budget = Some(p.replace("Budget:", "").trim().to_string());
                } else if lower.starts_with("location:") {
                    location = Some(p.replace("Location:", "").trim().to_string());
                }
            }
        }
    }
    if budget.is_none() {
        let budget_re = regex::Regex::new(r"(?i)budget:?\s*\$?([\d\w\s\.,\-]+)")
            .unwrap();
        if let Some(cap) = budget_re.captures(description) {
            let b = cap.get(1).map(|m| m.as_str().trim().to_string());
            if let Some(b) = b {
                if b.len() < 40 {
                    budget = Some(b.trim_end_matches('.').to_string());
                }
            }
        }
    }
    (budget, location)
}

fn keyword_relevant(tag: &str) -> bool {
    let t = tag.to_lowercase();
    let tech_terms = [
        "react", "reactjs", "node", "nodejs", "rust", "solidity", "python", "django", "flask",
        "javascript", "typescript", "frontend", "backend", "fullstack", "web", "api", "rest",
        "graphql", "sql", "database", "blockchain", "web3", "smart contract", "solana", "ethereum",
        "tailwind", "css", "html", "vue", "angular", "next.js", "cloud", "aws", "laravel", "php",
        "java", "spring", "go", "golang", "c++", "ai", "ml", "machine learning", "automation",
        "scraping", "data", "figma", "ui", "ux", "saas", "mobile", "app", "shopify", "wordpress",
    ];
    tech_terms.iter().any(|term| t.contains(term))
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(max).collect();
        t.push('…');
        t
    }
}

fn keyword_clean(title: &str) -> String {
    let re = regex::Regex::new(r"[^a-zA-Z0-9 ]").unwrap();
    re.replace_all(title, " ")
        .split_whitespace()
        .take(4)
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rss_items() {
        let rss = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
<channel>
  <title>Upwork</title>
  <item>
    <title>Build a react dashboard (Budget: $1500, Location: USA)</title>
    <link>https://www.upwork.com/jobs/~01abc</link>
    <description><![CDATA[<p>Need a responsive dashboard in React + Tailwind.</p>]]></description>
    <category>React</category>
    <category>Python</category>
  </item>
</channel>
</rss>"#;
        let leads = parse_rss(rss).unwrap();
        assert_eq!(leads.len(), 1);
        assert_eq!(leads[0].source, "upwork");
        assert!(leads[0].budget.as_deref().unwrap_or("").contains("$1500"));
        assert_eq!(leads[0].location.as_deref(), Some("USA"));
        assert!(leads[0].technologies.as_deref().unwrap_or("").contains("React"));
        assert!(!leads[0].description.contains("<p>"));
    }

    #[test]
    fn skips_empty_items() {
        let rss = r#"<rss><channel><item><title></title></item></channel></rss>"#;
        assert!(parse_rss(rss).unwrap().is_empty());
    }

    #[test]
    fn strips_html_entities() {
        assert_eq!(strip_html("<p>A &amp; B &#39;C&#39;</p>"), "A & B 'C'");
    }

    #[test]
    fn parses_embedded_job_json() {
        let html = r#"
<script>window.__JOB_POSTINGS_LIST_DATA__ = {"jobResults":[{"op_title":"Build a rust API","jobUri":"/jobs/~0123456789abcdef","budget":2500,"skills":["Rust","Backend"]}]};</script>
"#;
        let leads = parse_search_page(html).unwrap();
        assert_eq!(leads.len(), 1);
        assert_eq!(leads[0].title, "Build a rust API");
        assert!(leads[0].url.contains("/jobs/~0123456789abcdef"));
        assert_eq!(leads[0].budget_min, Some(2500.0));
        assert!(leads[0].technologies.as_deref().unwrap_or("").contains("Rust"));
    }

    #[test]
    fn parses_hourly_rate_range() {
        let job: serde_json::Value = serde_json::json!({
            "title": "Fullstack dev",
            "url": "/jobs/~x",
            "rate": { "min": 40, "max": 70 }
        });
        let lead = job_to_lead(&job, &["op_title", "title", "jobTitle"]).unwrap();
        assert!(lead.budget.as_deref().unwrap_or("").contains("$40/hr - $70/hr"));
        assert_eq!(lead.budget_min, Some(40.0));
        assert_eq!(lead.budget_max, Some(70.0));
    }

    #[test]
    fn search_page_fallback_extracts_links() {
        let html = r#"<a class="up-n-link job-tile-title" href="/jobs/~abc">Build a scraper</a> <a href="/jobs/~def">Python API</a>"#;
        let leads = parse_search_page(html).unwrap();
        assert!(!leads.is_empty());
        assert!(leads.iter().all(|l| l.source == "upwork"));
    }
}