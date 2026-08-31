use crate::db::Db;
use crate::models::{Lead, NewLead};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// User profile (used for scoring + outreach personalisation)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: Option<String>,
    pub title: Option<String>,
    pub email: Option<String>,
    pub location: Option<String>,
    pub rate: Option<String>,
    pub skills: Vec<String>,
    pub experience: Option<String>,
    pub availability: Option<String>,
    pub bio: Option<String>,
    pub portfolio: Option<String>,
    pub linkedin: Option<String>,
    pub github: Option<String>,
}

impl Profile {
    pub fn from_db(db: &Db) -> Self {
        Profile {
            name: db.get_setting("profile.name"),
            title: db.get_setting("profile.title"),
            email: db.get_setting("profile.email"),
            location: db.get_setting("profile.location"),
            rate: db.get_setting("profile.rate"),
            skills: db
                .get_setting("profile.skills")
                .unwrap_or_default()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            experience: db.get_setting("profile.experience"),
            availability: db.get_setting("profile.availability"),
            bio: db.get_setting("profile.bio"),
            portfolio: db.get_setting("profile.portfolio"),
            linkedin: db.get_setting("profile.linkedin"),
            github: db.get_setting("profile.github"),
        }
    }

    pub fn save(&self, db: &Db) -> Result<(), rusqlite::Error> {
        db.set_setting("profile.name", self.name.as_deref().unwrap_or(""))?;
        db.set_setting("profile.title", self.title.as_deref().unwrap_or(""))?;
        db.set_setting("profile.email", self.email.as_deref().unwrap_or(""))?;
        db.set_setting("profile.location", self.location.as_deref().unwrap_or(""))?;
        db.set_setting("profile.rate", self.rate.as_deref().unwrap_or(""))?;
        db.set_setting("profile.skills", &self.skills.join(", "))?;
        db.set_setting("profile.experience", self.experience.as_deref().unwrap_or(""))?;
        db.set_setting("profile.availability", self.availability.as_deref().unwrap_or(""))?;
        db.set_setting("profile.bio", self.bio.as_deref().unwrap_or(""))?;
        db.set_setting("profile.portfolio", self.portfolio.as_deref().unwrap_or(""))?;
        db.set_setting("profile.linkedin", self.linkedin.as_deref().unwrap_or(""))?;
        db.set_setting("profile.github", self.github.as_deref().unwrap_or(""))?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Scoring
// ---------------------------------------------------------------------------

/// Score a lead against the user profile (skills overlap + signals).
/// Returns a score between 0 and 100. Pure function so it is unit-testable.
pub fn score_lead_against_profile(score: i64, lead: &Lead, profile: &Profile) -> i64 {
    let mut s = score;

    // Skills overlap: the strongest driver of fit.
    if !profile.skills.is_empty() {
        let text = format!(
            "{} {} {}",
            lead.title,
            lead.description,
            lead.technologies.as_deref().unwrap_or("")
        )
        .to_lowercase();
        let mut matched = 0usize;
        for skill in &profile.skills {
            let sk = skill.trim().to_lowercase();
            if !sk.is_empty() && (text.contains(&sk) || skill_substring(&sk, &text)) {
                matched += 1;
            }
        }
        let fraction = matched as f64 / profile.skills.len() as f64;
        // up to +45 points for strong skills match
        s += (fraction * 45.0).round() as i64;
    }

    // Location fit (remote-friendly or exact location match).
    if let Some(loc) = &lead.location {
        let l = loc.to_lowercase();
        if l.contains("remote") {
            s += 10;
        } else if let Some(pl) = profile.location.as_deref() {
            if l.contains(&pl.to_lowercase()) {
                s += 10;
            }
        }
    }

    // Recency bonus.
    if let Some(pd) = &lead.posted_date {
        if posted_recently(pd) {
            s += 5;
        }
    }

    s.clamp(0, 100)
}

fn skill_substring(skill: &str, text: &str) -> bool {
    // e.g. "react.js" vs "react"
    let stem: String = skill.chars().filter(|c| c.is_alphanumeric()).collect();
    stem.len() >= 2 && text.contains(&stem)
}

fn posted_recently(date: &str) -> bool {
    // Accept RFC2822 (RSS pubDate), ISO8601 or simple dates; award recency if
    // within the last 14 days. Parse leniently.
    let now = chrono::Utc::now();
    let parsed = chrono::DateTime::parse_from_rfc2822(date)
        .map(|d| d.with_timezone(&chrono::Utc))
        .or_else(|_| chrono::DateTime::parse_from_rfc3339(date).map(|d| d.with_timezone(&chrono::Utc)));
    if let Ok(ts) = parsed {
        let age = now.signed_duration_since(ts);
        return age.num_days() <= 14;
    }
    // last-ditch: try date-only "YYYY-MM-DD"
    if let Ok(d) = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d") {
        let age = now.date_naive().signed_duration_since(d);
        return age.num_days() <= 14;
    }
    false
}

// ---------------------------------------------------------------------------
// Outreach generation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutreachDraft {
    pub medium: String, // "proposal" | "linkedin_message" | "email"
    pub subject: Option<String>,
    pub body: String,
}

/// Generate a personalised outreach draft for a lead. Pure, template-based so it
/// works offline (no LLM key needed), but reads the user's profile fields.
pub fn generate_outreach(lead: &Lead, profile: &Profile) -> Vec<OutreachDraft> {
    let name = profile.name.as_deref().unwrap_or("").to_string();
    let title = profile.title.as_deref().unwrap_or("freelance developer").to_string();
    let email = profile.email.clone().unwrap_or_default();
    let skills = if profile.skills.is_empty() {
        "my skills".to_string()
    } else {
        profile.skills.join(", ")
    };
    let rate = profile.rate.clone().unwrap_or_default();
    let portfolio = profile.portfolio.as_deref().unwrap_or("available on request").to_string();
    let experience = profile.experience.as_deref().unwrap_or("years of").to_string();
    let linkedin = profile.linkedin.as_deref().unwrap_or("available on request").to_string();
    let client = lead
        .client_name
        .as_deref()
        .filter(|c| !c.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| "Hiring team".to_string());
    let job = &lead.title;
    let budget_line = if let Some(b) = &lead.budget {
        if !b.trim().is_empty() {
            format!(" I see the posted budget is {}, and I can work within that.", b)
        } else {
            rate_hint(&rate)
        }
    } else {
        rate_hint(&rate)
    };

    // 1) Freelance proposal (Upwork/Fiverr style)
    let proposal = format!(
        "Hi {client},\n\nI'm {name}, a {title} with experience in {skills}. I read your posting for \"{job}\" and it is a strong match for what I do best.{budget_line}\n\nHere is how I'd approach it:\n- A clear plan and timeline up front\n- Open communication and regular progress updates\n- A focus on clean, maintainable, on-time delivery\n\nI'd love to jump on a quick call to confirm the scope. My portfolio: {portfolio}. Looking forward to working with you!\n\nBest regards,\n{name}",
    );

    // 2) LinkedIn message (short)
    let linkedin_message = format!(
        "Hi {client},\n\nI'm {name}, a {title}. I noticed you're hiring for \"{job}\" and wanted to introduce myself — my {skills} background looks like a good fit. Would you be open to a quick chat this week?\n\nBest,\n{name}"
    );

    // 3) Email (used in applications / direct outreach)
    let subject = format!("Application: {} — {}", job, title);
    let email_body = format!(
        "Dear {client},\n\nI'm applying for the {job} role. I am a {title} with {experience} working on {skills}.{budget_line}\n\nI would be glad to share samples of relevant work and to walk through how I'd add value from day one.\n\nHere is how to reach me:\n- Email: {email}\n- Portfolio: {portfolio}\n- LinkedIn: {linkedin}\n\nThank you for your time and consideration.\n\nBest regards,\n{name}",
    );

    vec![
        OutreachDraft {
            medium: "proposal".into(),
            subject: None,
            body: proposal,
        },
        OutreachDraft {
            medium: "linkedin_message".into(),
            subject: None,
            body: linkedin_message,
        },
        OutreachDraft {
            medium: "email".into(),
            subject: Some(subject),
            body: email_body,
        },
    ]
}

fn rate_hint(rate: &str) -> String {
    if rate.is_empty() {
        String::new()
    } else {
        format!(" My usual rate is {} and I am happy to align it with your budget.", rate)
    }
}

// ---------------------------------------------------------------------------
// LinkedIn OAuth handling
// ---------------------------------------------------------------------------

/// Returns the LinkedIn OAuth "Log in with LinkedIn" authorization URL.
/// The user completes this in their browser; LinkedIn redirects back with a
/// `code` that we exchange for an access token (see `exchange_linkedin_code`).
pub fn linkedin_auth_url(
    client_id: &str,
    redirect_uri: &str,
    state: &str,
    scope: &str,
) -> String {
    let redirect = urlencoding::encode(redirect_uri).into_owned();
    let state_enc = urlencoding::encode(state).into_owned();
    // LinkedIn uses space-separated scopes; urlencoding turns spaces into %20,
    // which is what LinkedIn expects in the query string.
    let scope_enc = urlencoding::encode(scope).into_owned();
    format!(
        "https://www.linkedin.com/oauth/v2/authorization?response_type=code&client_id={}&redirect_uri={}&state={}&scope={}",
        client_id, redirect, state_enc, scope_enc
    )
}

/// Exchange the LinkedIn authorization code for an access token.
/// Requires the app's client secret and the same redirect URI. Returns the raw
/// JSON (containing access_token, expires_in, scope).
pub async fn exchange_linkedin_code(
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
    code: &str,
) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();
    let url = "https://www.linkedin.com/oauth/v2/accessToken";
    let resp = client
        .post(url)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("client_id", client_id),
            ("client_secret", client_secret),
        ])
        .send()
        .await
        .map_err(|e| format!("request failed: {}", e))?;
    let status = resp.status();
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("bad json: {}", e))?;
    if !status.is_success() {
        return Err(format!("linkedin token error {}: {}", status, body));
    }
    Ok(body)
}

/// Fetch the current member's profile (used to confirm a successful connection).
pub async fn linkedin_me(token: &str) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://api.linkedin.com/v2/userinfo")
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| format!("request failed: {}", e))?;
    let status = resp.status();
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("bad json: {}", e))?;
    if !status.is_success() {
        return Err(format!("userinfo error {}: {}", status, body));
    }
    Ok(body)
}

/// Convert a LinkedIn `code` into a stored token and the member's display name
/// in one step. Saves the token (and expiry) to settings so later flows can use
/// it without the user re-authorizing.
pub async fn connect_linkedin(
    db: &Db,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
    code: &str,
) -> Result<serde_json::Value, String> {
    let body = exchange_linkedin_code(client_id, client_secret, redirect_uri, code).await?;
    let token = body
        .get("access_token")
        .and_then(|t| t.as_str())
        .ok_or_else(|| "no access_token in response".to_string())?
        .to_string();
    let expires = body
        .get("expires_in")
        .and_then(|e| e.as_i64())
        .unwrap_or(0);
    db.set_setting("linkedin.access_token", &token)
        .map_err(|e| format!("db: {}", e))?;
    db.set_setting("linkedin.expires_at", &format!("{}", expires))
        .map_err(|e| format!("db: {}", e))?;
    if let Ok(info) = linkedin_me(&token).await {
        if let Some(sub) = info.get("sub").and_then(|s| s.as_str()) {
            db.set_setting("linkedin.member_id", sub)
                .map_err(|e| format!("db: {}", e))?;
        }
        if let Some(n) = info.get("name").and_then(|n| n.as_str()) {
            db.set_setting("linkedin.member_name", n)
                .map_err(|e| format!("db: {}", e))?;
        }
    }
    Ok(body)
}

// ---------------------------------------------------------------------------
// Manual lead helpers
// ---------------------------------------------------------------------------

/// Build a lead from a hand-pasted job/gig URL. Because job sites block scraping
/// of logged-out pages, we cannot fetch reliable details, so we store the URL
/// with a friendly placeholder and mark it for review. The user can enrich it
/// (title/notes) in the UI.
pub fn lead_from_url(url: &str) -> NewLead {
    let url = url.trim().to_string();
    let source = guess_source(&url);
    let title = format!("{} opportunity — needs review", display_source(&source));
    NewLead {
        source,
        title,
        description: "Pasted manually. Add the role details and your fit notes, then run scoring.".to_string(),
        url,
        budget: None,
        budget_min: None,
        budget_max: None,
        currency: Some("USD".into()),
        location: None,
        technologies: None,
        client_name: None,
        posted_date: None,
    }
}

fn guess_source(url: &str) -> String {
    let u = url.to_lowercase();
    if u.contains("upwork") {
        "upwork".into()
    } else if u.contains("fiverr") {
        "fiverr".into()
    } else if u.contains("freelancer") {
        "freelancer".into()
    } else if u.contains("indeed") {
        "indeed".into()
    } else if u.contains("linkedin") {
        "linkedin".into()
    } else if u.contains("facebook") || u.contains("fb.com") || u.contains("facebook.com") {
        "facebook".into()
    } else {
        "manual".into()
    }
}

fn display_source(source: &str) -> &str {
    match source {
        "upwork" => "Upwork",
        "fiverr" => "Fiverr",
        "freelancer" => "Freelancer",
        "indeed" => "Job",
        "linkedin" => "LinkedIn",
        "facebook" => "Facebook",
        _ => "Lead",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;

    fn profile() -> Profile {
        Profile {
            name: Some("Aarav".into()),
            title: Some("Full-Stack Developer".into()),
            email: Some("aarav@example.com".into()),
            location: Some("Remote".into()),
            rate: Some("$50/hr".into()),
            skills: vec!["React".into(), "Rust".into(), "Python".into()],
            experience: Some("5+".into()),
            availability: Some("Full-time".into()),
            bio: Some("I build products.".into()),
            portfolio: Some("example.com".into()),
            linkedin: Some("linkedin.com/in/aarav".into()),
            github: Some("github.com/aarav".into()),
        }
    }

    fn lead() -> Lead {
        Lead {
            id: 1,
            source: "indeed".into(),
            title: "Senior React Engineer".into(),
            description: "Build front-end features with React and TypeScript.".into(),
            url: "https://indeed.com/viewjob?jk=1".into(),
            budget: Some("$120k".into()),
            budget_min: None,
            budget_max: None,
            currency: Some("USD".into()),
            location: Some("Remote".into()),
            technologies: Some("React, TypeScript".into()),
            client_name: Some("Acme".into()),
            posted_date: None,
            status: "new".into(),
            score: 0,
            notes: None,
            created_at: "2026-08-31".into(),
        }
    }

    #[test]
    fn scores_skills_overlap() {
        let p = profile();
        let l = lead();
        let s = score_lead_against_profile(0, &l, &p);
        // React (1 of 3 skills) + Remote should score above zero and below max.
        assert!(s > 20, "expected decent score, got {}", s);
        assert!(s <= 100);
    }

    #[test]
    fn full_skill_match_scores_high() {
        let p = profile();
        let l = Lead {
            title: "Full stack role: React + Rust + Python".into(),
            description: "Prefer someone strong in React front-end, Rust services and Python tooling.".into(),
            technologies: Some("React, Rust, Python".into()),
            ..lead()
        };
        let s = score_lead_against_profile(0, &l, &p);
        assert!(s >= 50, "expected high score, got {}", s);
    }

    #[test]
    fn unrelated_lead_scores_low() {
        let p = profile();
        let l = Lead {
            title: "Accountant for taxes".into(),
            description: "Bookkeeping, QuickBooks, compliance.".into(),
            technologies: Some("Excel".into()),
            ..lead()
        };
        let s = score_lead_against_profile(0, &l, &p);
        assert!(s < 15, "expected low score, got {}", s);
    }

    #[test]
    fn generates_outreach_with_profile() {
        let p = profile();
        let l = lead();
        let drafts = generate_outreach(&l, &p);
        assert_eq!(drafts.len(), 3);
        let email_subject = drafts.iter().find(|d| d.medium == "email").unwrap();
        assert!(email_subject.subject.as_deref().unwrap().contains(&l.title));
        assert!(email_subject.body.contains("Aarav"));
        assert!(email_subject.body.contains("React"));
        assert!(email_subject.body.contains("Acme"));
    }

    #[test]
    fn builds_linkedin_auth_url() {
        let url = linkedin_auth_url("cid", "http://localhost:5173/callback", "state123", "openid profile email");
        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id=cid"));
        assert!(url.contains("redirect_uri=http%3A%2F%2Flocalhost%3A5173%2Fcallback"));
        assert!(url.contains("state=state123"));
    }

    #[test]
    fn guesses_sources_and_builds_manual_lead() {
        assert_eq!(guess_source("https://www.upwork.com/jobs/~x"), "upwork");
        assert_eq!(guess_source("https://www.facebook.com/groups/123"), "facebook");
        assert_eq!(guess_source("https://www.indeed.com/viewjob?jk=9"), "indeed");
        let l = lead_from_url("https://www.linkedin.com/jobs/view/123");
        assert_eq!(l.source, "linkedin");
        assert!(l.url.contains("linkedin"));
    }

    #[test]
    fn profile_roundtrip() {
        let db = Db::new(":memory:").unwrap();
        let p = profile();
        p.save(&db).unwrap();
        let loaded = Profile::from_db(&db);
        assert_eq!(loaded.name.as_deref(), Some("Aarav"));
        assert_eq!(loaded.skills, vec!["React", "Rust", "Python"]);
        assert_eq!(loaded.rate.as_deref(), Some("$50/hr"));
    }
}
