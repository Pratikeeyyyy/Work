use crate::models::NewLead;
use crate::scraper::http_client;
use urlencoding::encode;

// Fiverr heavily protects its search pages with JS + anti-bot.
// This collector tries the public gigs search endpoint and gracefully
// falls back with a hint when it is blocked.
pub async fn fetch(keyword: &str) -> Result<Vec<NewLead>, String> {
    // Public search page (server-rendered shell). Actual gigs are in __NEXT_DATA__.
    let url = format!("https://www.fiverr.com/search/gigs?query={}", encode(keyword));
    let client = http_client().map_err(|e| format!("http client: {}", e))?;
    let resp = client
        .get(&url)
        .header("X-Requested-With", "XMLHttpRequest")
        .send()
        .await
        .map_err(|e| format!("request failed: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!(
            "Fiverr blocked this request (status {}). Add manual leads with the URL of gigs you find.",
            resp.status()
        ));
    }
    let body = resp.text().await.map_err(|e| format!("read body: {}", e))?;
    parse(&body)
}

fn parse(body: &str) -> Result<Vec<NewLead>, String> {
    let mut leads = Vec::new();
    extract_json_blob(body, ">>`window.Fiverr :", "\n", &mut leads);
    if leads.is_empty() {
        // attempt simple regex scan for gig title/link patterns
        let title_re = regex::Regex::new(r#""title":"([^"]{5,140})"#).unwrap();
        let desc_re = regex::Regex::new(r#""description":"(.*?)","#).unwrap();
        let link_re = regex::Regex::new(r#""url":"(/[A-Za-z0-9\-_\.]+/[A-Za-z0-9\-_\.]+)"#).unwrap();
        let titles: Vec<String> = title_re
            .captures_iter(body)
            .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
            .take(20)
            .collect();
        let links: Vec<String> = link_re
            .captures_iter(body)
            .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
            .take(20)
            .collect();
        for (i, title) in titles.into_iter().enumerate() {
            let link = links
                .get(i)
                .cloned()
                .unwrap_or_default();
            leads.push(NewLead {
                source: "fiverr".into(),
                title: title.clone(),
                description: desc_re
                    .captures_iter(body)
                    .nth(i)
                    .and_then(|c| c.get(1))
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| "Gig found on Fiverr — open the link to see full details.".into())
                    .replace("\\n", " ")
                    .replace("\\\"", "\""),
                url: if link.starts_with('/') {
                    format!("https://www.fiverr.com{}", link)
                } else {
                    link
                },
                budget: None,
                budget_min: None,
                budget_max: None,
                currency: Some("USD".into()),
                location: None,
                technologies: Some(keyword_clean(&title)),
                client_name: None,
                posted_date: None,
            });
        }
    }
    Ok(leads)
}

// minimal JSON marker extraction (kept intentionally simple)
fn extract_json_blob(
    _body: &str,
    _marker: &str,
    _end: &str,
    _out: &mut Vec<NewLead>,
) {
    // Reserved for deeper __NEXT_DATA__ parsing if Fiverr changes layout.
}

fn keyword_clean(title: &str) -> String {
    let re = regex::Regex::new(r"[^a-zA-Z0-9 ]").unwrap();
    re.replace_all(title, " ")
        .split_whitespace()
        .take(4)
        .collect::<Vec<_>>()
        .join(", ")
}