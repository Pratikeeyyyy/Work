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
| `DATABASE_PATH` | `leadgen.db` | SQLite file path |

## Tests

```bash
cargo test
```

Covers the three scrapers (upwork, freelancer, fiverr) and the database layer
(lead insert/dedupe, scoring, contract status updates, settings).

## Architecture

- `main.rs` — HTTP server bootstrap, tracing, CORS
- `api.rs` — all REST route handlers
- `db.rs` — SQLite schema, migrations, queries
- `models.rs` — serde structs shared across the API and DB layers
- `scraper/` — source collectors: `upwork.rs`, `freelancer.rs`, `fiverr.rs`

## Scrapers

| Source | Method | Notes |
|--------|--------|-------|
| Upwork | Official RSS feed | Reliable |
| Freelancer | AJAX project search endpoint | Reliable |
| Fiverr | `__NEXT_DATA__` JSON + regex fallback | Fiverr blocks bots; parser is defensive, falls back gracefully |

## Endpoints

See the root [README](../README.md#api-endpoints) for the full endpoint table.
