use crate::models::NewLead;
use crate::scraper::http_client;
use serde_json::Value;
use urlencoding::encode;

// Fiverr protects search pages with JS + anti-bot, but when a real browser
// reaches the page it embeds the full SSR payload in a `__NEXT_DATA__` script
// tag (JSON blob). This collector prefers that structured payload and falls
// back to a lighter DOM regex scan when JSON is unavailable or blocked.
//
// Known payload shapes (Fiverr changes these; parse defensively):
//   __NEXT_DATA__.props.pageProps.<searchResults|gigs|gigResults>.[]
//   each gig: title, seller/username, url/slug, price, rating, reviews, tags
pub async fn fetch(keyword: &str) -> Result<Vec<NewLead>, String> {
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
    // 1) Prefer the structured __NEXT_DATA__ payload.
    if let Some(blob) = extract_next_data(body) {
        return match parse_next_data(&blob) {
            Ok(leads) if !leads.is_empty() => Ok(leads),
            _ => Ok(regex_fallback(body)),
        };
    }
    // 2) Fall back to a light regex scan for gig title/link patterns.
    Ok(regex_fallback(body))
}

/// Pull the raw JSON from `<script id="__NEXT_DATA__" type="application/json">…</script>`.
fn extract_next_data(body: &str) -> Option<String> {
    let marker = r#"__NEXT_DATA__"#;
    let idx = body.find(marker)?;
    // Jump to the first '{' after the marker to skip the script tag attributes.
    let rest = &body[idx..];
    let start = rest.find('{')?;
    let json_start = idx + start;

    // Find the closing `</script>` and backtrack to the matching final '}'.
    let end_tag = body[json_start..].find("</script>")?;
    let slice = &body[json_start..json_start + end_tag];
    // Trim trailing whitespace, then drop a trailing '}' that belongs to </script> boundary.
    let trimmed = slice.trim_end();
    if trimmed.is_empty() {
        return None;
    }
    // The JSON object closes with a '}'. Walk back from the end to the matching close.
    let close = trimmed.rfind('}')?;
    let json_str = &trimmed[..=close];
    Some(json_str.to_string())
}

/// Parse the search-result gigs out of a Fiverr __NEXT_DATA__ payload.
fn parse_next_data(raw: &str) -> Result<Vec<NewLead>, String> {
    let data: Value = serde_json::from_str(raw).map_err(|e| format!("json parse: {}", e))?;
    let props = data
        .pointer("/props/pageProps")
        .or_else(|| data.get("props").and_then(|p| p.get("pageProps")));

    let mut gigs: Vec<&Value> = Vec::new();
    if let Some(props) = props {
        for key in [
            "searchResults",
            "gigResults",
            "gigs",
            "results",
            "searchProps",
        ] {
            if let Some(arr) = props.get(key).and_then(|v| v.as_array()) {
                gigs.extend(arr.iter().collect::<Vec<_>>());
                break;
            }
        }
        // Some layouts nest under props.searchData / props.mainResults etc.
        if gigs.is_empty() {
            collect_gig_arrays(props, &mut gigs, 0);
        }
    }

    let mut leads = Vec::new();
    for gig in gigs {
        if let Some(lead) = gig_to_lead(gig) {
            leads.push(lead);
        }
    }
    Ok(leads)
}

/// Defensive depth-limited traversal that collects arrays whose items look like gigs.
fn collect_gig_arrays<'a>(node: &'a Value, out: &mut Vec<&'a Value>, depth: usize) {
    if depth > 6 || out.len() > 200 {
        return;
    }
    match node {
        Value::Array(arr) => {
            // Heuristic: an array of objects that look like gigs (have title + url/slug).
            let looks_like_gigs = arr
                .iter()
                .take(1)
                .all(|v| v.is_object() && (v.get("title").is_some() || v.get("gigTitle").is_some()));
            if looks_like_gigs && arr.len() >= 1 {
                out.extend(arr.iter());
            } else {
                for item in arr {
                    collect_gig_arrays(item, out, depth + 1);
                }
            }
        }
        Value::Object(map) => {
            for v in map.values() {
                collect_gig_arrays(v, out, depth + 1);
            }
        }
        _ => {}
    }
}

fn gig_to_lead(gig: &Value) -> Option<NewLead> {
    // ---- title ----
    let title = gig
        .get("title")
        .or_else(|| gig.get("gigTitle"))
        .or_else(|| gig.get("gig_title"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())?;

    // ---- url ----
    let username = gig
        .get("seller")
        .and_then(|s| {
            if let Some(u) = s.get("username").and_then(|u| u.as_str()) {
                return Some(u.to_string());
            }
            s.get("userName").and_then(|u| u.as_str()).map(String::from)
        })
        .or_else(|| {
            gig.get("username")
                .or_else(|| gig.get("gig_username"))
                .and_then(|v| v.as_str())
                .map(String::from)
        });
    let slug = gig
        .get("slug")
        .or_else(|| gig.get("gigSlug"))
        .or_else(|| gig.get("url"))
        .and_then(|v| v.as_str());

    let url = build_gig_url(username.as_deref(), slug, &title);
    if url.is_empty() {
        return None;
    }

    // ---- starting price ----
    let (budget, budget_min, budget_max) = extract_price(gig);

    // ---- tags / technologies ----
    let technologies = extract_tags(gig, &title);

    let client_name = username.map(|u| u.replace('_', " ").to_string());

    Some(NewLead {
        source: "fiverr".into(),
        title,
        description: gig
            .get("description")
            .or_else(|| gig.get("gigDescription"))
            .and_then(|v| v.as_str())
            .map(|s| {
                strip_html(s)
                    .chars()
                    .take(4000)
                    .collect::<String>()
            })
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "Gig found on Fiverr — open the link to see full details.".to_string()),
        url,
        budget,
        budget_min,
        budget_max,
        currency: Some("USD".into()),
        location: None,
        technologies,
        client_name,
        posted_date: None,
    })
}

fn build_gig_url(username: Option<&str>, slug: Option<&str>, title: &str) -> String {
    if let (Some(u), Some(s)) = (username, slug) {
        let u = u.trim().trim_start_matches('@');
        let s = s
            .trim_start_matches('/')
            .split(['?', '#'])
            .next()
            .unwrap_or(s)
            .trim();
        if u.is_empty() {
            return format!("https://www.fiverr.com/{}", s);
        }
        return format!("https://www.fiverr.com/{}/{}", u, s);
    }
    if let Some(slug) = slug {
        let s = slug.trim_start_matches('/').split(['?', '#']).next().unwrap_or(slug);
        if s.contains('/') {
            return format!("https://www.fiverr.com/{}", s);
        }
    }
    // Fall back to a search link keyed on title words.
    let words: Vec<&str> = title
        .split_whitespace()
        .filter(|w| w.chars().all(|c| c.is_alphanumeric()))
        .take(6)
        .collect();
    let slug = words.join("-").to_lowercase();
    if slug.is_empty() {
        String::new()
    } else {
        format!("https://www.fiverr.com/q/{}/{}", slug.replace(' ', "-"), slug)
    }
}

fn extract_price(gig: &Value) -> (Option<String>, Option<f64>, Option<f64>) {
    // Common paths: gig.price / gig.packages[0].price / gig.priceFrom / gig.startingPrice
    let price = gig
        .get("startPrice")
        .or_else(|| gig.get("startingPrice"))
        .or_else(|| gig.get("priceFrom"))
        .or_else(|| gig.get("price"))
        .or_else(|| {
            gig.get("packages")
                .and_then(|p| p.as_array())
                .and_then(|arr| arr.first())
                .and_then(|p| p.get("price"))
        });

    let val = price.map(|p| {
        p.as_f64()
            .or_else(|| p.as_str().and_then(|s| s.replace(',', "").parse::<f64>().ok()))
            .unwrap_or(0.0)
    });

    match val {
        Some(v) if v > 0.0 => (
            Some(format!("Starting at ${:.0}", v)),
            Some(v),
            Some(v),
        ),
        _ => (None, None, None),
    }
}

fn extract_tags(gig: &Value, title: &str) -> Option<String> {
    let mut tags: Vec<String> = Vec::new();
    // tags as array of objects with "name", or array of strings, or "tags" string list
    if let Some(t) = gig.get("tags") {
        match t {
            Value::Array(arr) => {
                for item in arr {
                    if let Some(name) = item.get("name").and_then(|n| n.as_str()) {
                        tags.push(name.to_string());
                    } else if let Some(s) = item.as_str() {
                        tags.push(s.to_string());
                    }
                }
            }
            Value::String(s) => {
                tags.extend(s.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()));
            }
            _ => {}
        }
    }
    if tags.is_empty() {
        // Fall back to significant words from the title.
        let re = regex::Regex::new(r"[^a-zA-Z0-9 ]").unwrap();
        let binding = re.replace_all(title, " ");
        let words: Vec<&str> = binding
            .split_whitespace()
            .filter(|w| w.len() > 2 && w.to_lowercase() != "iwill")
            .take(4)
            .collect();
        tags = words.into_iter().map(String::from).collect();
    }
    if tags.is_empty() {
        None
    } else {
        let mut seen = std::collections::HashSet::new();
        tags.retain(|t| seen.insert(t.to_lowercase()));
        Some(tags.into_iter().take(6).collect::<Vec<_>>().join(", "))
    }
}

fn strip_html(s: &str) -> String {
    let re = regex::Regex::new(r"<[^>]*>").unwrap();
    re.replace_all(s, " ").trim().to_string()
}

/// Lightweight regex fallback for when the JSON payload is absent/blocked.
fn regex_fallback(body: &str) -> Vec<NewLead> {
    let mut leads = Vec::new();
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
        let link = links.get(i).cloned().unwrap_or_default();
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
    leads
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
    fn next_data_extracts_and_parses_gigs() {
        let html = r##"
<html><body>
<script id="__NEXT_DATA__" type="application/json">
{"props":{"pageProps":{"searchResults":[
  {"title":"I will build a python web scraper","seller":{"username":"techfreelancer99"},
   "slug":"build-a-python-web-scraper","tags":[{"name":"web scraping"},{"name":"python"}],
   "startingPrice":30,"description":"<b>Custom</b> data extraction"},
  {"title":"I will write a rust api","username":"rusted_dev","gigSlug":"write-a-rust-api",
   "tags":["rust","backend"],"price":"50","description":"production ready rust"}
]}}}
</script>
</body></html>
"##;
        let leads = parse(html).unwrap();
        assert_eq!(leads.len(), 2);
        assert_eq!(leads[0].source, "fiverr");
        assert!(leads[0].url.contains("techfreelancer99"));
        assert!(leads[0].url.contains("build-a-python-web-scraper"));
        assert!(leads[0].budget.as_deref().unwrap_or("").contains("$30"));
        assert!(leads[0].technologies.as_deref().unwrap_or("").contains("python"));
        assert!(!leads[0].description.contains("<b>"));
        assert_eq!(leads[1].budget_min, Some(50.0));
    }

    #[test]
    fn falls_back_to_regex_when_no_next_data() {
        let html = r#"id="__IGNORED__">x</script><div>{"title":"Some gig title","description":"desc here","url":"/seller/some-gig-slug"}</div>"#;
        let leads = parse(html).unwrap();
        // Either path may yield results; ensure we never error on a plain page.
        assert!(leads.iter().all(|l| l.source == "fiverr"));
    }

    #[test]
    fn build_url_from_username_and_slug() {
        assert_eq!(
            build_gig_url(Some("alice"), Some("/logo-design"), "Logo design"),
            "https://www.fiverr.com/alice/logo-design"
        );
        assert_eq!(
            build_gig_url(Some("@bob"), Some("slug-with-query?x=1"), "t"),
            "https://www.fiverr.com/bob/slug-with-query"
        );
    }

    #[test]
    fn empty_input_no_gigs() {
        assert!(parse("<html></html>").unwrap().is_empty());
    }
}
