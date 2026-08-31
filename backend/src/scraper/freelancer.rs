use crate::models::NewLead;
use crate::scraper::http_client;
use urlencoding::encode;

// Freelancer.com exposes an AJAX project search endpoint used by its own site.
// This collector hits it politely; if it is blocked it explains how to fall back.
pub async fn fetch(keyword: &str) -> Result<Vec<NewLead>, String> {
    let url = format!(
        "https://www.freelancer.com/ajax/search/projects.php?query={}&limit=30&offset=0&state=all",
        encode(keyword)
    );
    let client = http_client().map_err(|e| format!("http client: {}", e))?;
    let resp = client
        .get(&url)
        .header("X-Requested-With", "XMLHttpRequest")
        .header("Accept", "application/json, text/javascript, */*; q=0.01")
        .send()
        .await
        .map_err(|e| format!("request failed: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!(
            "Freelancer blocked this request (status {}). Add manual leads with project URLs instead.",
            resp.status()
        ));
    }
    let body = resp.text().await.map_err(|e| format!("read body: {}", e))?;
    parse(&body)
}

fn parse(body: &str) -> Result<Vec<NewLead>, String> {
    // The endpoint returns JSON with a `projects` object keyed by id:
    // { "projects": { "123": { "title": "...", "description": "...",
    //     "bid_stats": { "bid_count": n }, "job_url": "slug/123", "budget": { "minimum": x, "maximum": y, "currency": "US" }, ... } } }
    let json: serde_json::Value =
        serde_json::from_str(body).map_err(|e| format!("json parse: {}", e))?;
    let projects = json.get("projects").and_then(|p| p.as_object());
    let mut leads = Vec::new();
    if let Some(projects) = projects {
        for (_id, proj) in projects {
            let title = proj.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled project");
            let desc = proj
                .get("description")
                .and_then(|v| v.as_str())
                .map(strip_tags)
                .unwrap_or_default();
            let slug = proj
                .get("job_url")
                .or_else(|| proj.get("seo_url"))
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let url = format!("https://www.freelancer.com/projects/{}-{}", slug, _id);
            let (budget_min, budget_max, currency) = extract_budget(proj);
            let (country, username) = extract_client(proj);

            let budget = match (budget_min, budget_max) {
                (Some(a), Some(b)) => Some(format!("${:.0} - ${:.0}", a, b)),
                (Some(a), None) => Some(format!("${:.0}+", a)),
                (None, Some(b)) => Some(format!("up to ${:.0}", b)),
                (None, None) => None,
            };

            leads.push(NewLead {
                source: "freelancer".into(),
                title: title.trim().to_string(),
                description: truncate(&desc, 4000),
                url,
                budget,
                budget_min,
                budget_max,
                currency: Some(currency.unwrap_or("USD".into())),
                location: country,
                technologies: proj
                    .get("skills")
                    .and_then(|s| s.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|s| s.get("name").and_then(|n| n.as_str()))
                            .take(6)
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .filter(|s| !s.is_empty())
                    .or_else(|| Some(keyword_from_title(&title))),
                client_name: username.map(|u| u.replace('_', " ").to_string()),
                posted_date: proj
                    .get("time_updated")
                    .or_else(|| proj.get("time_submitted"))
                    .and_then(|v| v.as_i64())
                    .map(|ts| ts_to_date(ts as i64)),
            });
        }
    }
    Ok(leads)
}

fn extract_budget(proj: &serde_json::Value) -> (Option<f64>, Option<f64>, Option<String>) {
    let budget = proj.get("budget");
    let min = budget
        .and_then(|b| b.get("minimum"))
        .and_then(|v| v.as_f64());
    let max = budget
        .and_then(|b| b.get("maximum"))
        .and_then(|v| v.as_f64());
    let currency = budget
        .and_then(|b| b.get("currency"))
        .and_then(|v| v.as_str())
        .map(currency_name);
    (min, max, currency)
}

fn extract_client(proj: &serde_json::Value) -> (Option<String>, Option<String>) {
    let location = proj
        .get("owner")
        .and_then(|o| o.get("country"))
        .and_then(|v| v.get("name"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let username = proj
        .get("owner")
        .and_then(|o| o.get("username"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    (location, username)
}

fn strip_tags(s: &str) -> String {
    let re = regex::Regex::new(r"<[^>]*>").unwrap();
    re.replace_all(s, " ").trim().to_string()
}

fn keyword_from_title(title: &str) -> String {
    let re = regex::Regex::new(r"[^a-zA-Z0-9 ]").unwrap();
    let binding = re.replace_all(title, " ");
    let words: Vec<&str> = binding
        .split_whitespace()
        .filter(|w| w.len() > 2 && w.to_lowercase() != "need")
        .take(4)
        .collect();
    words.join(", ")
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

fn ts_to_date(ts: i64) -> String {
    use chrono::TimeZone;
    chrono::Utc
        .timestamp_opt(ts, 0)
        .single()
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_default()
}

fn currency_name(code: &str) -> String {
    match code {
        "US" | "USD" => "USD".to_string(),
        "EUR" => "EUR".to_string(),
        "GBP" => "GBP".to_string(),
        "AU" | "AUD" => "AUD".to_string(),
        "CA" | "CAD" => "CAD".to_string(),
        "IN" | "INR" => "INR".to_string(),
        other => other.to_string(),
    }
}