use crate::models::NewLead;
use crate::scraper::http_client;
use crate::scraper::strip_html;
use serde::Deserialize;
use urlencoding::encode;

// Remotive: legal public JSON API of remote jobs. No auth, no scraping.
// https://remotive.com/api/remote-jobs?search=<kw>
// Response: { "jobs": [ { url, title, company_name, candidate_required_location,
//                         salary, publication_date, description, tags, ... } ] }

#[derive(Deserialize)]
struct RemotiveJob {
    url: String,
    title: String,
    #[serde(default)]
    company_name: String,
    #[serde(default)]
    candidate_required_location: String,
    #[serde(default)]
    salary: String,
    #[serde(default)]
    publication_date: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Deserialize)]
struct RemotiveResponse {
    #[serde(default)]
    jobs: Vec<RemotiveJob>,
}

pub async fn fetch(keyword: &str) -> Result<Vec<NewLead>, String> {
    let url = format!(
        "https://remotive.com/api/remote-jobs?search={}",
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
    let data: RemotiveResponse =
        serde_json::from_str(body).map_err(|e| format!("json: {}", e))?;
    Ok(data
        .jobs
        .into_iter()
        .filter_map(|j| {
            if j.url.is_empty() || j.title.trim().is_empty() {
                return None;
            }
            Some(NewLead {
                source: "remotive".into(),
                title: j.title.trim().to_string(),
                description: strip_html(&j.description),
                url: j.url,
                budget: {
                    let s = j.salary.trim().to_string();
                    if s.is_empty() {
                        None
                    } else {
                        Some(s)
                    }
                },
                budget_min: None,
                budget_max: None,
                currency: None,
                location: {
                    let l = j.candidate_required_location.trim().to_string();
                    if l.is_empty() {
                        None
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
                client_name: {
                    let c = j.company_name.trim().to_string();
                    if c.is_empty() {
                        None
                    } else {
                        Some(c)
                    }
                },
                posted_date: {
                    let d = j.publication_date.trim().to_string();
                    if d.is_empty() {
                        None
                    } else {
                        Some(d)
                    }
                },
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_jobs_json() {
        let body = r#"{"jobs":[
            {"url":"https://remotive.com/jobs/1","title":"React Engineer","company_name":"Acme","candidate_required_location":"Remote","tags":["react","rust"],"publication_date":"2026-08-01T00:00:00","salary":"$80k","description":"<p>Build React apps &amp; Rust</p>"}
        ]}"#;
        let leads = parse(body).unwrap();
        assert_eq!(leads.len(), 1);
        assert_eq!(leads[0].source, "remotive");
        assert_eq!(leads[0].title, "React Engineer");
        assert_eq!(leads[0].client_name.as_deref(), Some("Acme"));
        assert_eq!(leads[0].technologies.as_deref(), Some("react, rust"));
        assert_eq!(leads[0].budget.as_deref(), Some("$80k"));
        assert!(leads[0].description.contains("Build React apps"));
        assert!(!leads[0].description.contains('<'));
    }

    #[test]
    fn skips_empty_jobs() {
        let body = r#"{"jobs":[{"url":"","title":""}]}"#;
        assert!(parse(body).unwrap().is_empty());
    }
}
