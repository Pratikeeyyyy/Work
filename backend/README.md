# LeadGen Backend

Rust API server for the LeadGen platform. Serves the lead-generation pipeline and escrow contract management.

## Stack

- **Rust** (edition 2021)
- **Axum** 0.7 web framework
- **SQLite** via `rusqlite` (bundled)
- **reqwest** for scraping, **rss** for Upwork feeds

## Run

```bash
cargo run
```

Serves on `:8080` (override with `BIND` env var). Creates `leadgen.db` on first run.

## Environment

| Variable | Default | Description |
|----------|---------|-------------|
| `BIND` | `0.0.0.0:8080` | Listen address |
| `DATABASE_PATH` | `leadgen.db` | SQLite file path (account registry; each user also gets a `user_<name>.db` data file in the same directory) |
| `CORS_ORIGIN` | `http://localhost:5173` | Allowed frontend origin (comma-separated allowed) |

## Tests

```bash
cargo test
```

Covers the source scrapers (remotive, weworkremotely, remoteok, indeed, upwork,
freelancer, fiverr) and the database layer (lead insert/dedupe, scoring, contract
status updates, settings, user registration + per-user data isolation).

## Architecture

- `main.rs` — HTTP server bootstrap, tracing, CORS, and the background auto-discovery worker (`auto_discovery_loop`)
- `auth.rs` — multi-user auth: PBKDF2 password hashing, user-scoped in-memory sessions, auth middleware
- `api.rs` — all REST route handlers
- `db.rs` — SQLite schema, migrations, queries
- `models.rs` — serde structs shared across the API and DB layers
- `scraper/` — source collectors: `upwork.rs`, `freelancer.rs`, `fiverr.rs`, `indeed.rs`
- `hunt.rs` — profile fit-scoring and outreach generation

## Auto-discovery

A background task polls the reliable Indeed RSS feed on a configured interval,
scores newly inserted leads against the user profile, and marks any lead at/above
the fit threshold as `queued` for the Discover page. Tuned via settings:
`auto_pull_enabled`, `auto_pull_interval_mins`, `auto_queue_threshold`. Disabled
by default.

## Scrapers

| Source | Method | Notes |
|--------|--------|-------|
| Upwork | Legacy RSS + search-page fallback (`__JOB_POSTINGS_LIST_DATA__`/anchor scan) | Upwork discontinued public RSS (Aug 2024) and bot-blocks scrapers; parser degrades to a source error and recommends manual lead capture |
| Freelancer | AJAX project search endpoint | Reliable |
| Fiverr | `__NEXT_DATA__` JSON + regex fallback | Fiverr blocks bots; parser is defensive, falls back gracefully |

## Endpoints

See the root [README](../README.md#api-endpoints) for the full endpoint table.
