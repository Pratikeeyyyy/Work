use crate::models::NewLead;
use crate::scraper::http_client;
use urlencoding::encode;

// Indeed supports an openly documented RSS job feed that (unlike Upwork/Fiverr/
// Freelancer) does not require authentication or a browser session, which makes
// it a reliable first-class source for full-time/remote job hunting.
// Feed: https://www.indeed.com/rss?q=<query>&l=<location>
pub async fn fetch(keyword: &str, location: Option<&str>) -> Result<Vec<NewLead>, String> {
    let mut url = format!("https://www.indeed.com/rss?q={}", encode(keyword));
    if let Some(loc) = location {
        if !loc.trim().is_empty() {
            url.push_str("&l=");
            url.push_str(&encode(loc.trim()));
        }
    }
    let client = http_client().map_err(|e| format!("http client: {}", e))?;
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("request failed: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("status {}", resp.status()));
    }
    let body = resp
        .text()
        .await
        .map_err(|e| format!("read body: {}", e))?;
    parse(&body, keyword)
}

fn parse(body: &str, keyword: &str) -> Result<Vec<NewLead>, String> {
    let feed = rss::Channel::read_from(body.as_bytes()).map_err(|e| format!("rss parse: {}", e))?;
    let mut leads = Vec::new();
    for item in feed.items() {
        let title = item.title().unwrap_or("").trim().to_string();
        if title.is_empty() {
            continue;
        }
        let link = item.link().unwrap_or("").to_string();
        let description = strip_html(item.description().unwrap_or(""));
        let company = guess_company(&description).or_else(|| guess_company(&title));
        let location = guess_location(&description);
        let tech = keyword_clean(keyword, &title);
        let posted_date = item.pub_date().map(|s| s.to_string());
        leads.push(NewLead {
            source: "indeed".into(),
            title,
            description: truncate(&description, 4000),
            url: link,
            budget: None,
            budget_min: None,
            budget_max: None,
            currency: Some("USD".into()),
            location,
            technologies: Some(tech),
            client_name: company,
            posted_date,
        });
    }
    Ok(leads)
}

fn strip_html(input: &str) -> String {
    let re = regex::Regex::new(r"<[^>]*>").unwrap();
    let stripped = re.replace_all(input, " ");
    let amp = regex::Regex::new(r"&amp;").unwrap();
    let lt = regex::Regex::new(r"&lt;").unwrap();
    let gt = regex::Regex::new(r"&gt;").unwrap();
    let quot = regex::Regex::new(r"&quot;").unwrap();
    let apos = regex::Regex::new(r"&#39;|&apos;").unwrap();
    let nbsp = regex::Regex::new(r"&nbsp;").unwrap();
    let mut s = stripped.to_string();
    for re in [amp, lt, gt, quot, apos, nbsp] {
        s = re.replace_all(&s, "").to_string();
    }
    s.split_whitespace().collect::<Vec<_>>().join(" ")
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

// Indeed feeds often include "Company - Location" in the title or description.
fn guess_company(input: &str) -> Option<String> {
    // Look for a likely company name preceding a location in the title pattern
    // e.g. "Fullstack Engineer  - Acme Corp  - New York, NY"
    let re = regex::Regex::new(r"(?i)\s+-\s+([A-Za-z0-9&'. ]{2,60})\s+-\s+[A-Za-z]").unwrap();
    if let Some(cap) = re.captures(input) {
        let raw = cap[1].trim();
        if !raw.is_empty() && raw.chars().count() <= 60 {
            return Some(raw.to_string());
        }
    }
    None
}

fn guess_location(input: &str) -> Option<String> {
    let re = regex::Regex::new(r"(?i)(remote|new york|los angeles|san francisco|austin|seattle|chicago|boston|denver|[\w\s]+,\s*[A-Z]{2})\b").unwrap();
    re.captures(input)
        .map(|c| c[0].trim().to_string())
        .filter(|s| s.chars().count() <= 80)
}

fn keyword_clean(keyword: &str, title: &str) -> String {
    let mut terms = vec![keyword.trim().to_string()];
    let re = regex::Regex::new(r"[^a-zA-Z0-9 ]").unwrap();
    let t = re.replace_all(title, " ");
    for word in t.split_whitespace().take(3) {
        if word.to_lowercase() != keyword.trim().to_lowercase() {
            terms.push(word.to_string());
        }
    }
    terms
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_indeed_rss_items() {
        let rss = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
<channel>
  <item>
    <title>Fullstack Engineer - Acme Corp - New York, NY</title>
    <link>https://www.indeed.com/viewjob?jk=abc123</link>
    <description>Acme Corp is hiring a fullstack engineer. Remote OK.</description>
  </item>
</channel>
</rss>"#;
        let leads = parse(rss, "react").unwrap();
        assert_eq!(leads.len(), 1);
        assert_eq!(leads[0].source, "indeed");
        assert_eq!(leads[0].client_name.as_deref(), Some("Acme Corp"));
        assert!(leads[0].technologies.as_deref().unwrap().contains("react"));
        assert!(leads[0].url.contains("viewjob"));
    }

    #[test]
    fn empty_feed_returns_empty() {
        let rss = r#"<rss><channel></channel></rss>"#;
        assert!(parse(rss, "rust").unwrap().is_empty());
    }

    #[test]
    fn detects_remote_and_company() {
        assert!(guess_location("Remote - anywhere").unwrap().contains("Remote"));
        assert_eq!(guess_company("Engineer - Big Tech Co - Austin, TX").unwrap(), "Big Tech Co");
    }
}
