use crate::models::*;
use rusqlite::{params, Connection, OptionalExtension};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Db {
    pub conn: Arc<Mutex<Connection>>,
}

impl Db {
    pub fn new(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS leads (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                source TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                url TEXT UNIQUE,
                budget TEXT,
                budget_min REAL,
                budget_max REAL,
                currency TEXT,
                location TEXT,
                technologies TEXT,
                client_name TEXT,
                posted_date TEXT,
                status TEXT NOT NULL DEFAULT 'new',
                score INTEGER NOT NULL DEFAULT 0,
                notes TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE INDEX IF NOT EXISTS idx_leads_status ON leads(status);
            CREATE INDEX IF NOT EXISTS idx_leads_source ON leads(source);
            CREATE INDEX IF NOT EXISTS idx_leads_score ON leads(score);

            CREATE TABLE IF NOT EXISTS clients (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                lead_id INTEGER REFERENCES leads(id),
                name TEXT NOT NULL,
                email TEXT,
                company TEXT,
                country TEXT,
                website TEXT,
                whatsapp TEXT,
                source TEXT,
                linkedin TEXT,
                past_work TEXT,
                preferences TEXT,
                status TEXT NOT NULL DEFAULT 'active',
                notes TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS contracts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                client_id INTEGER NOT NULL REFERENCES clients(id),
                client_address TEXT,
                freelancer_address TEXT,
                contract_address TEXT,
                title TEXT NOT NULL,
                amount_wei TEXT,
                currency TEXT NOT NULL DEFAULT 'ETH',
                status TEXT NOT NULL DEFAULT 'draft',
                tx_hash TEXT,
                deployed_at TEXT,
                notes TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS applications (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                lead_id INTEGER NOT NULL REFERENCES leads(id),
                client_id INTEGER REFERENCES clients(id),
                status TEXT NOT NULL DEFAULT 'saved',
                applied_at TEXT,
                replied_at TEXT,
                interviewed_at TEXT,
                offered_at TEXT,
                hired_at TEXT,
                company TEXT,
                contact TEXT,
                next_scheduled TEXT,
                follow_up_count INTEGER NOT NULL DEFAULT 0,
                last_follow_up TEXT,
                notes TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE INDEX IF NOT EXISTS idx_app_lead ON applications(lead_id);

            INSERT OR IGNORE INTO settings (key, value)
            VALUES ('keywords', 'web development, react, rust, python, blockchain, solidity'),
                   ('sources', 'upwork,freelancer,fiverr,indeed'),
                   ('max_leads_per_run', '100');
            "#,
        )?;
        Self::migrate(&conn)?;
        Ok(Db {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn migrate(conn: &Connection) -> rusqlite::Result<()> {
        let cols: Vec<String> = conn
            .prepare("SELECT name FROM pragma_table_info('contracts')")?
            .query_map([], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        if !cols.iter().any(|c| c == "contract_address") {
            conn.execute("ALTER TABLE contracts ADD COLUMN contract_address TEXT", [])?;
        }
        Ok(())
    }

    fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap()
    }

    // ---------- Leads ----------

    pub fn insert_lead(&self, l: &NewLead) -> Result<bool, rusqlite::Error> {
        let conn = self.conn();
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM leads WHERE url = ?1)",
                params![l.url],
                |r| r.get(0),
            )
            .unwrap_or(false);
        if exists {
            return Ok(false);
        }
        conn.execute(
            "INSERT INTO leads (source, title, description, url, budget, budget_min, budget_max, currency, location, technologies, client_name, posted_date, score)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
            params![
                l.source,
                l.title,
                l.description,
                l.url,
                l.budget,
                l.budget_min,
                l.budget_max,
                l.currency,
                l.location,
                l.technologies,
                l.client_name,
                l.posted_date,
                score_lead(l),
            ],
        )?;
        Ok(true)
    }

    pub fn list_leads(
        &self,
        source: Option<&str>,
        status: Option<&str>,
        query: Option<&str>,
        limit: i64,
    ) -> Result<Vec<Lead>, rusqlite::Error> {
        let conn = self.conn();
        let mut sql = String::from(
            "SELECT id, source, title, description, url, budget, budget_min, budget_max, currency, location, technologies, client_name, posted_date, status, score, notes, created_at
             FROM leads WHERE 1=1",
        );
        let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(s) = source {
            sql.push_str(" AND source = ?");
            args.push(Box::new(s.to_string()));
        }
        if let Some(st) = status {
            sql.push_str(" AND status = ?");
            args.push(Box::new(st.to_string()));
        }
        if let Some(q) = query {
            sql.push_str(" AND (title LIKE ? OR description LIKE ? OR technologies LIKE ? OR client_name LIKE ?)");
            let pat = format!("%{}%", q);
            args.push(Box::new(pat.clone()));
            args.push(Box::new(pat.clone()));
            args.push(Box::new(pat.clone()));
            args.push(Box::new(pat));
        }
        sql.push_str(" ORDER BY score DESC, created_at DESC LIMIT ?");
        args.push(Box::new(limit));

        let mut stmt = conn.prepare(&sql)?;
        let iter = {
            let params = rusqlite::params_from_iter(args.iter().map(|a| a.as_ref()));
            stmt.query_map(params, lead_from_row)?
        };
        let mut out = Vec::new();
        for row in iter {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn get_lead(&self, id: i64) -> Result<Option<Lead>, rusqlite::Error> {
        let conn = self.conn();
        conn.query_row(
            "SELECT id, source, title, description, url, budget, budget_min, budget_max, currency, location, technologies, client_name, posted_date, status, score, notes, created_at FROM leads WHERE id = ?1",
            params![id],
            lead_from_row,
        )
        .optional()
    }

    pub fn update_lead_status(&self, id: i64, status: &str) -> Result<(), rusqlite::Error> {
        let conn = self.conn();
        conn.execute(
            "UPDATE leads SET status = ?1 WHERE id = ?2",
            params![status, id],
        )?;
        Ok(())
    }

    pub fn update_lead_notes(&self, id: i64, notes: &str) -> Result<(), rusqlite::Error> {
        let conn = self.conn();
        conn.execute(
            "UPDATE leads SET notes = ?1 WHERE id = ?2",
            params![notes, id],
        )?;
        Ok(())
    }

    pub fn update_lead_score(&self, id: i64, score: i64) -> Result<(), rusqlite::Error> {
        let conn = self.conn();
        conn.execute(
            "UPDATE leads SET score = ?1 WHERE id = ?2",
            params![score, id],
        )?;
        Ok(())
    }

    pub fn delete_lead(&self, id: i64) -> Result<(), rusqlite::Error> {
        let conn = self.conn();
        conn.execute("DELETE FROM leads WHERE id = ?1", params![id])?;
        Ok(())
    }

    // ---------- Clients ----------

    pub fn insert_client(&self, c: &NewClient) -> Result<i64, rusqlite::Error> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO clients (lead_id, name, email, company, country, website, whatsapp, source, linkedin, past_work, preferences)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![
                c.lead_id,
                c.name,
                c.email,
                c.company,
                c.country,
                c.website,
                c.whatsapp,
                c.source,
                c.linkedin,
                c.past_work,
                c.preferences
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn client_from_lead(&self, lead_id: i64) -> Result<i64, rusqlite::Error> {
        let conn = self.conn();
        let existing: Option<i64> = conn
            .query_row(
                "SELECT id FROM clients WHERE lead_id = ?1",
                params![lead_id],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(id) = existing {
            return Ok(id);
        }
        let lead = self.get_lead(lead_id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        let name = lead
            .client_name
            .clone()
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| "Client from lead".to_string());
        conn.execute(
            "INSERT INTO clients (lead_id, name, company, source, notes, preferences)
             VALUES (?1,?2,?3,?4,?5,?6)",
            params![
                lead_id,
                name,
                lead.client_name,
                lead.source,
                lead.notes,
                lead.technologies
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list_clients(&self, status: Option<&str>) -> Result<Vec<Client>, rusqlite::Error> {
        let conn = self.conn();
        let mut sql = String::from("SELECT id, lead_id, name, email, company, country, website, whatsapp, source, linkedin, past_work, preferences, status, notes, created_at, updated_at FROM clients WHERE 1=1");
        let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(s) = status {
            sql.push_str(" AND status = ?");
            args.push(Box::new(s.to_string()));
        }
        sql.push_str(" ORDER BY updated_at DESC");
        let mut stmt = conn.prepare(&sql)?;
        let iter = {
            let params = rusqlite::params_from_iter(args.iter().map(|a| a.as_ref()));
            stmt.query_map(params, client_from_row)?
        };
        let mut out = Vec::new();
        for row in iter {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn get_client(&self, id: i64) -> Result<Option<Client>, rusqlite::Error> {
        let conn = self.conn();
        conn.query_row(
            "SELECT id, lead_id, name, email, company, country, website, whatsapp, source, linkedin, past_work, preferences, status, notes, created_at, updated_at FROM clients WHERE id = ?1",
            params![id],
            client_from_row,
        )
        .optional()
    }

    pub fn update_client(&self, c: &Client) -> Result<(), rusqlite::Error> {
        let conn = self.conn();
        conn.execute(
            "UPDATE clients SET name=?1, email=?2, company=?3, country=?4, website=?5, whatsapp=?6, source=?7, linkedin=?8, past_work=?9, preferences=?10, status=?11, notes=?12, updated_at=datetime('now') WHERE id=?13",
            params![
                c.name,
                c.email,
                c.company,
                c.country,
                c.website,
                c.whatsapp,
                c.source,
                c.linkedin,
                c.past_work,
                c.preferences,
                c.status,
                c.notes,
                c.id
            ],
        )?;
        Ok(())
    }

    pub fn delete_client(&self, id: i64) -> Result<(), rusqlite::Error> {
        let conn = self.conn();
        conn.execute("DELETE FROM clients WHERE id = ?1", params![id])?;
        Ok(())
    }

    // ---------- Contracts ----------

    pub fn insert_contract(&self, c: &NewContract) -> Result<i64, rusqlite::Error> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO contracts (client_id, client_address, freelancer_address, contract_address, title, amount_wei, currency, notes)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                c.client_id,
                c.client_address,
                c.freelancer_address,
                c.contract_address,
                c.title,
                c.amount_wei,
                c.currency,
                c.notes
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list_contracts(&self) -> Result<Vec<Contract>, rusqlite::Error> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, client_id, client_address, freelancer_address, contract_address, title, amount_wei, currency, status, tx_hash, deployed_at, notes, created_at FROM contracts ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], contract_from_row)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn update_contract_deployment(
        &self,
        id: i64,
        status: &str,
        tx_hash: &str,
        contract_address: Option<&str>,
    ) -> Result<(), rusqlite::Error> {
        let conn = self.conn();
        conn.execute(
            "UPDATE contracts SET status=?1, tx_hash=?2, contract_address=?3, deployed_at=datetime('now') WHERE id=?4",
            params![status, tx_hash, contract_address, id],
        )?;
        Ok(())
    }

    pub fn update_contract_status(&self, id: i64, status: &str) -> Result<(), rusqlite::Error> {
        let conn = self.conn();
        conn.execute(
            "UPDATE contracts SET status=?1 WHERE id=?2",
            params![status, id],
        )?;
        Ok(())
    }

    // ---------- Applications ----------

    pub fn add_application(&self, a: &NewApplication) -> Result<i64, rusqlite::Error> {
        let conn = self.conn();
        let existing: Option<i64> = conn
            .query_row(
                "SELECT id FROM applications WHERE lead_id = ?1",
                params![a.lead_id],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(id) = existing {
            return Ok(id);
        }
        conn.execute(
            "INSERT INTO applications (lead_id, client_id, company, contact, notes)
             VALUES (?1,?2,?3,?4,?5)",
            params![a.lead_id, a.client_id, a.company, a.contact, a.notes],
        )?;
        // Mark the underlying lead as "applied" so dashboard stats stay in sync.
        conn.execute(
            "UPDATE leads SET status = 'applied' WHERE id = ?1",
            params![a.lead_id],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list_applications(&self) -> Result<Vec<Application>, rusqlite::Error> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT a.id, a.lead_id, a.client_id, a.status, a.applied_at, a.replied_at,
                    a.interviewed_at, a.offered_at, a.hired_at, a.company, a.contact,
                    a.next_scheduled, a.follow_up_count, a.last_follow_up, a.notes, a.created_at,
                    l.title, l.url, l.source
             FROM applications a LEFT JOIN leads l ON l.id = a.lead_id
             ORDER BY COALESCE(a.applied_at, a.created_at) DESC",
        )?;
        let rows = stmt.query_map([], app_from_row)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn get_application(&self, id: i64) -> Result<Option<Application>, rusqlite::Error> {
        let conn = self.conn();
        conn.query_row(
            "SELECT a.id, a.lead_id, a.client_id, a.status, a.applied_at, a.replied_at,
                    a.interviewed_at, a.offered_at, a.hired_at, a.company, a.contact,
                    a.next_scheduled, a.follow_up_count, a.last_follow_up, a.notes, a.created_at,
                    l.title, l.url, l.source
             FROM applications a LEFT JOIN leads l ON l.id = a.lead_id WHERE a.id = ?1",
            params![id],
            app_from_row,
        )
        .optional()
    }

    pub fn update_application(
        &self,
        id: i64,
        u: &ApplicationUpdate,
    ) -> Result<(), rusqlite::Error> {
        let conn = self.conn();
        let cur = self.get_application(id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        let status = u.status.clone().unwrap_or_else(|| cur.status.clone());

        let field = |v: &Option<String>| v.clone();
        let applied_at = field(&u.applied_at).or(cur.applied_at.clone());
        let replied_at = field(&u.replied_at).or(cur.replied_at.clone());
        let interviewed_at = field(&u.interviewed_at).or(cur.interviewed_at.clone());
        let offered_at = field(&u.offered_at).or(cur.offered_at.clone());
        let hired_at = field(&u.hired_at).or(cur.hired_at.clone());
        let company = field(&u.company).or(cur.company.clone());
        let contact = field(&u.contact).or(cur.contact.clone());
        let next_scheduled = field(&u.next_scheduled).or(cur.next_scheduled.clone());
        let notes = field(&u.notes).or(cur.notes.clone());

        // When moving to a pipeline stage, stamp the timestamp automatically.
        let now = chrono::Utc::now().naive_utc().to_string();
        let applied_at = if status == "applied" && applied_at.is_none() {
            Some(now.clone())
        } else {
            applied_at
        };
        let replied_at = if status == "replied" && replied_at.is_none() {
            Some(now.clone())
        } else {
            replied_at
        };
        let interviewed_at = if status == "interviewed" && interviewed_at.is_none() {
            Some(now.clone())
        } else {
            interviewed_at
        };
        let offered_at = if status == "offered" && offered_at.is_none() {
            Some(now.clone())
        } else {
            offered_at
        };
        let hired_at = if status == "hired" && hired_at.is_none() {
            Some(now.clone())
        } else {
            hired_at
        };

        let follow_up_count = if u.follow_up {
            cur.follow_up_count + 1
        } else {
            cur.follow_up_count
        };
        let last_follow_up = if u.follow_up {
            Some(now.clone())
        } else {
            cur.last_follow_up.clone()
        };

        conn.execute(
            "UPDATE applications SET status=?1, applied_at=?2, replied_at=?3, interviewed_at=?4,
                    offered_at=?5, hired_at=?6, company=?7, contact=?8, next_scheduled=?9,
                    notes=?10, follow_up_count=?11, last_follow_up=?12 WHERE id=?13",
            params![
                status,
                applied_at,
                replied_at,
                interviewed_at,
                offered_at,
                hired_at,
                company,
                contact,
                next_scheduled,
                notes,
                follow_up_count,
                last_follow_up,
                id
            ],
        )?;
        // Sync the lead status with the application stage.
        conn.execute(
            "UPDATE leads SET status = ?1 WHERE id = ?2",
            params![status, cur.lead_id],
        )?;
        Ok(())
    }

    pub fn delete_application(&self, id: i64) -> Result<(), rusqlite::Error> {
        let conn = self.conn();
        conn.execute("DELETE FROM applications WHERE id = ?1", params![id])?;
        Ok(())
    }

    // ---------- Settings ----------

    pub fn get_setting(&self, key: &str) -> Option<String> {
        let conn = self.conn();
        conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |r| r.get(0),
        )
        .ok()
    }

    pub fn get_keywords(&self) -> Vec<String> {
        self.get_setting("keywords")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), rusqlite::Error> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    // ---------- Stats ----------

    pub fn stats(&self) -> Result<Stats, rusqlite::Error> {
        let conn = self.conn();
        let one = |sql: &str| -> i64 {
            conn.query_row(sql, [], |r| r.get(0)).unwrap_or(0)
        };
        let by_source: Vec<SourceCount> = {
            let mut stmt = conn.prepare(
                "SELECT source, COUNT(*) as c FROM leads GROUP BY source ORDER BY c DESC",
            )?;
            let rows = stmt.query_map([], |r| {
                Ok(SourceCount {
                    source: r.get(0)?,
                    count: r.get(1)?,
                })
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };
        let top_technologies: Vec<TechCount> = {
            // flatten comma-separated technologies column
            let mut stmt = conn.prepare("SELECT technologies FROM leads WHERE technologies IS NOT NULL")?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
            let mut map: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
            for tech in rows.flatten() {
                for t in tech.split(',').map(|s| s.trim()) {
                    if !t.is_empty() {
                        *map.entry(t.to_lowercase()).or_insert(0) += 1;
                    }
                }
            }
            let mut v: Vec<TechCount> = map
                .into_iter()
                .map(|(tech, count)| TechCount { tech, count })
                .collect();
            v.sort_by(|a, b| b.count.cmp(&a.count));
            v.truncate(10);
            v
        };
        Ok(Stats {
            total_leads: one("SELECT COUNT(*) FROM leads"),
            new_leads: one("SELECT COUNT(*) FROM leads WHERE status = 'new'"),
            applied_leads: one("SELECT COUNT(*) FROM leads WHERE status IN ('applied','shortlisted','responded')"),
            won_leads: one("SELECT COUNT(*) FROM leads WHERE status = 'won'"),
            total_clients: one("SELECT COUNT(*) FROM clients"),
            active_clients: one("SELECT COUNT(*) FROM clients WHERE status != 'archived'"),
            total_contracts: one("SELECT COUNT(*) FROM contracts"),
            total_applications: one("SELECT COUNT(*) FROM applications"),
            interviewed: one("SELECT COUNT(*) FROM applications WHERE status IN ('interviewed','offered','hired')"),
            hired: one("SELECT COUNT(*) FROM applications WHERE status = 'hired'"),
            by_source,
            top_technologies,
        })
    }
}

fn lead_from_row(row: &rusqlite::Row) -> rusqlite::Result<Lead> {
    Ok(Lead {
        id: row.get(0)?,
        source: row.get(1)?,
        title: row.get(2)?,
        description: row.get(3)?,
        url: row.get(4)?,
        budget: row.get(5)?,
        budget_min: row.get(6)?,
        budget_max: row.get(7)?,
        currency: row.get(8)?,
        location: row.get(9)?,
        technologies: row.get(10)?,
        client_name: row.get(11)?,
        posted_date: row.get(12)?,
        status: row.get(13)?,
        score: row.get(14)?,
        notes: row.get(15)?,
        created_at: row.get(16)?,
    })
}

fn client_from_row(row: &rusqlite::Row) -> rusqlite::Result<Client> {
    Ok(Client {
        id: row.get(0)?,
        lead_id: row.get(1)?,
        name: row.get(2)?,
        email: row.get(3)?,
        company: row.get(4)?,
        country: row.get(5)?,
        website: row.get(6)?,
        whatsapp: row.get(7)?,
        source: row.get(8)?,
        linkedin: row.get(9)?,
        past_work: row.get(10)?,
        preferences: row.get(11)?,
        status: row.get(12)?,
        notes: row.get(13)?,
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
    })
}

fn contract_from_row(row: &rusqlite::Row) -> rusqlite::Result<Contract> {
    Ok(Contract {
        id: row.get(0)?,
        client_id: row.get(1)?,
        client_address: row.get(2)?,
        freelancer_address: row.get(3)?,
        contract_address: row.get(4)?,
        title: row.get(5)?,
        amount_wei: row.get(6)?,
        currency: row.get(7)?,
        status: row.get(8)?,
        tx_hash: row.get(9)?,
        deployed_at: row.get(10)?,
        notes: row.get(11)?,
        created_at: row.get(12)?,
    })
}

fn app_from_row(row: &rusqlite::Row) -> rusqlite::Result<Application> {
    Ok(Application {
        id: row.get(0)?,
        lead_id: row.get(1)?,
        client_id: row.get(2)?,
        status: row.get(3)?,
        applied_at: row.get(4)?,
        replied_at: row.get(5)?,
        interviewed_at: row.get(6)?,
        offered_at: row.get(7)?,
        hired_at: row.get(8)?,
        company: row.get(9)?,
        contact: row.get(10)?,
        next_scheduled: row.get(11)?,
        follow_up_count: row.get(12)?,
        last_follow_up: row.get(13)?,
        notes: row.get(14)?,
        created_at: row.get(15)?,
        lead_title: row.get(16)?,
        lead_url: row.get(17)?,
        lead_source: row.get(18)?,
    })
}

fn score_lead(l: &NewLead) -> i64 {
    let mut score = 0;
    if l.budget_min.is_some() || l.budget_max.is_some() {
        score += 20;
    }
    if let Some(budget) = &l.budget {
        score += 10;
        let upper = parse_budget_number(budget);
        match upper {
            Some(v) => {
                if v >= 1000.0 {
                    score += 20;
                } else if v >= 500.0 {
                    score += 10;
                }
            }
            None => {}
        }
    }
    if l.client_name.is_some() {
        score += 10;
    }
    if l.location.is_some() {
        score += 5;
    }
    if let Some(tech) = &l.technologies {
        let t = tech.to_lowercase();
        for kw in ["urgent", "asap", "immediately", "today", "now"] {
            if t.contains(kw) {
                score += 5;
            }
        }
    }
    score
}

fn parse_budget_number(budget: &str) -> Option<f64> {
    let cleaned: String = budget
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
        .collect();
    let parts: Vec<&str> = cleaned.split('-').collect();
    if let Some(last) = parts.last() {
        return last.trim().parse::<f64>().ok();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Db {
        Db::new(":memory:").expect("in-memory db")
    }

    fn sample_lead() -> NewLead {
        NewLead {
            source: "upwork".into(),
            title: "Build a react dashboard".into(),
            description: "Need a responsive dashboard".into(),
            url: "https://upwork.com/jobs/~test".into(),
            budget: Some("$500 - $1500".into()),
            budget_min: Some(500.0),
            budget_max: Some(1500.0),
            currency: Some("USD".into()),
            location: Some("USA".into()),
            technologies: Some("React".into()),
            client_name: Some("Acme".into()),
            posted_date: None,
        }
    }

    #[test]
    fn inserts_and_dedupes_leads_by_url() {
        let db = test_db();
        assert!(db.insert_lead(&sample_lead()).unwrap());
        // Same URL -> not inserted again.
        assert!(!db.insert_lead(&sample_lead()).unwrap());
        assert_eq!(db.list_leads(None, None, None, 100).unwrap().len(), 1);
    }

    #[test]
    fn scores_leads() {
        let l = sample_lead();
        assert!(score_lead(&l) > 0);
        let empty = NewLead {
            budget: None,
            budget_min: None,
            budget_max: None,
            client_name: None,
            location: None,
            technologies: None,
            ..sample_lead()
        };
        assert_eq!(score_lead(&empty), 0);
    }

    #[test]
    fn contract_status_update() {
        let db = test_db();
        let client = NewClient {
            lead_id: None,
            name: "Alice".into(),
            email: None,
            company: None,
            country: None,
            website: None,
            whatsapp: None,
            source: None,
            linkedin: None,
            past_work: None,
            preferences: None,
        };
        let client_id = db.insert_client(&client).unwrap();
        let c = NewContract {
            client_id,
            client_address: Some("0xclient".into()),
            freelancer_address: Some("0xfreelancer".into()),
            contract_address: None,
            title: "Milestone 1".into(),
            amount_wei: Some("1000000000000000000".into()),
            currency: "ETH".into(),
            notes: None,
        };
        let id = db.insert_contract(&c).unwrap();
        db.update_contract_status(id, "funded").unwrap();
        let contracts = db.list_contracts().unwrap();
        assert_eq!(contracts[0].status, "funded");
    }

    #[test]
    fn settings_roundtrip_and_defaults() {
        let db = test_db();
        assert!(!db.get_keywords().is_empty());
        db.set_setting("keywords", "rust, solidity").unwrap();
        assert_eq!(db.get_keywords(), vec!["rust".to_string(), "solidity".to_string()]);
    }
}