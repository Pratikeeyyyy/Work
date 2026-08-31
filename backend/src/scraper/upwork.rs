use crate::models::NewLead;
use crate::scraper::http_client;
use urlencoding::encode;

// Upwork publishes official public RSS job feeds.
// URL: https://www.upwork.com/ab/feed/jobs/rss?keywords=<kw>&sort=recency&...
pub async fn fetch(keyword: &str) -> Result<Vec<NewLead>, String> {
    let url = format!(
        "https://www.upwork.com/ab/feed/jobs/rss?keywords={}&sort=recency&job_type=billing&payment_verified=1&budget=500-100000&paging=0&api_full_job_description=1",
        encode(keyword)
    );
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
    parse(&body)
}

fn parse(body: &str) -> Result<Vec<NewLead>, String> {
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

        // Try to infer a client/company name from the link path
        let client_name = guess_client_name(&title);

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
            client_name,
            posted_date: pub_date,
        });
    }
    Ok(leads)
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

fn guess_client_name(title: &str) -> Option<String> {
    // strip parenthetical meta and leading "Urgent" etc.
    let without_meta = title.split('(').next().unwrap_or(title);
    let w = without_meta.trim();
    if w.chars().count() < 4 {
        return None;
    }
    None
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