# LeadGen — Project State

> Last updated: 2026-08-31

## Git status

- Branch: `main`, clean tree, pushed to `origin/main`
- Remote: `Pratikeeyyyy/Work`

## What's done

### contracts/
- `FreelanceEscrow.sol` — full lifecycle (Funded → InProgress → Submitted → Completed / Disputed / Refunded)
- 9 passing Hardhat tests
- Deploy scripts for localhost/sepolia/mainnet
- ABI artifact copied to `frontend/src/lib/FreelanceEscrow.json`
- **Not deployed to any live network** (no `.env` configured)

### backend/
- Rust/Axum REST API on `:8080`, SQLite (`leadgen.db`)
- Full CRUD for leads, clients, contracts
- Scrape endpoint with Upwork, Freelancer, Fiverr (fragile) + **Indeed RSS**
- Deploy endpoint records tx_hash + contract_address
- `PATCH /contracts/:id/status` — lifecycle status updates (deployed/funded/in_progress/submitted/completed/disputed/refunded)
- **Job-hunt automation (hunt.rs)**: Profile (load/save via settings), `score_lead_against_profile` (skills + location + recency, 0–100), `generate_outreach` (proposal / linkedin_message / email from profile), LinkedIn OAuth helpers (`auth_url`, `callback`, `me`, `connect`), `lead_from_url`/`guess_source`
- **Applications DB** (`applications` table): CRUD, stage auto-timestamping, follow-up counts, lead status sync, dedupe by lead
- **New endpoints**: `/leads/import`, `/leads/rescore`, `/leads/:id/outreach`, `/applications` CRUD, `/profile` GET/PUT, `/settings/linkedin`, `/linkedin/auth-url|callback|status`
- `cargo test`: **27 passing** (Indeed parse/remote tests, hunt scoring/outreach/profile/LinkedIn tests, applications)

### frontend/
- React + Vite + Tailwind on `:5173`
- Pages: Dashboard, Leads, **Applications**, Clients, Contracts, Settings
- Wallet integration (MetaMask via ethers v6)
- Deploy escrow from wallet (one-shot)
- **Escrow lifecycle UX (done)**: Contracts page shows on-chain state (via `getEscrowInfo`), role-based action buttons (Start work / Submit work / Approve & pay / Raise dispute / Cancel & refund / Refund after deadline / Resolve dispute with 50/50 split), on-chain info panel (state, deposit, deadline, mediator)
- **Settings**: My profile form (skills/location/rate/bio/etc for scoring + outreach) + LinkedIn OAuth app config + connect button + keywords/sources sections
- **Leads**: Import-URL modal, Generate-outreach modal (3 copy-paste drafts), Track-application action, rescore
- **Applications page**: pipeline cards with saved→applied→replied→interviewed→offered→hired stage stepper, reject/close, follow-up logging, notes/next-step editing
- `npm run build` (typecheck + vite) green; `npx vitest run`: **19 passing**

## Completed: Escrow lifecycle UX

### Backend
- `db.rs`: added `update_contract_status(id, status)`
- `api.rs`: added `PATCH /contracts/:id/status` route + handler (validates statuses, exists check)

### Frontend
- `lib/escrow.ts`: added `getEscrowInfo` + all action wrappers (`startWork`, `submitWork`, `approveWork`, `cancelBeforeWork`, `raiseDispute`, `refundAfterDeadline`, `resolveDispute`) + `EscrowInfo` type + `EscrowState` constants
- `api.ts`: added `updateContractStatus(id, status)`
- `components/Badge.tsx`: added `in_progress` (violet), `submitted` (amber) tones
- `pages/Contracts.tsx`: on-chain state loader, `EscrowActions` role/state-aware buttons, `OnChainInfo` panel, `DisputeModal` (mediator split)

## Completed: Fiverr scraper + backend tests

### Fiverr scraper (`backend/src/scraper/fiverr.rs`)
- Rewrote `extract_json_blob` placeholder into a real `__NEXT_DATA__` parser:
  - `extract_next_data()` — pulls JSON from `<script id="__NEXT_DATA__">`
  - `parse_next_data()` — reads `props.pageProps.searchResults|gigResults|gigs|results` plus a defensive recursive array scan
  - `gig_to_lead()` — maps title, seller url, starting price, tags/tech, description
  - Kept the regex fallback for when the JSON payload is absent or blocked
- Note: Fiverr returns an anti-bot challenge ("It needs a human touch") to non-browser requests, so live parsing can't be verified end-to-end; the parser is validated by unit tests with realistic fixtures and still degrades gracefully to the regex fallback.

### Backend tests (`cargo test`, 14 passing)
- `scraper/fiverr.rs`: 4 tests (extract+parse, regex fallback, url building, empty input)
- `scraper/upwork.rs`: 3 tests (RSS parse, empty skip, html stripping)
- `scraper/freelancer.rs`: 3 tests (JSON projects parse, empty, tag stripping)
- `db.rs`: 4 tests (insert+dedupe, scoring, contract status update, settings roundtrip)

## Completed: Frontend tests + component READMEs

### Frontend tests (`npm run test`, 19 passing via Vitest + RTL)
- `lib/format.test.ts` — formatWei, shortAddress, joinTags, timeAgo
- `lib/escrow.test.ts` — explorerUrl builders
- `components/Badge.test.tsx` — render + statusTone/displayLabel
- Setup: `src/test/setup.ts` with jsdom env, `test` script in package.json

### Component READMEs
- `backend/README.md`, `frontend/README.md`, `contracts/README.md`

## Completed: Upwork scraper resilience + E2E verification

### Upwork scraper (`backend/src/scraper/upwork.rs`)
- Discovered Upwork **discontinued public RSS feeds (Aug 2024)**; the legacy `/ab/feed/jobs/rss` endpoint now returns `410 Gone` without auth.
- Refactored `fetch()` to try the legacy RSS first (harmless if it ever works), then fall back to scraping the public search page:
  - `parse_search_page()` — extracts embedded `window.__JOB_POSTINGS_LIST_DATA__` via `parse_embedded_json()` (recursive array scan), with an anchor/link regex fallback for layout drift.
  - `job_to_lead()` / `extract_budget_fields()` — maps titles, urls, fixed budgets and hourly rate ranges, skills.
- All three job sites bot-block non-browser requests, so scrapes surface as source errors (surfaced through the `errors` array) instead of crashing — the live pipeline was verified to handle this gracefully.
- `cargo test`: now **17 passing** (5 new Upwork tests: embedded JSON, hourly rate, search-page fallback, RSS, meta).

### End-to-end API verification (in-memory SQLite, live binary)
Ran the running backend on `127.0.0.1:8091` and exercised the full flow:
- `POST /clients` → created client id 1
- `POST /contracts` → created contract
- `POST /contracts/1/deploy` → recorded tx_hash + contract_address
- `PATCH /contracts/1/status` → walked funded → in_progress → submitted → completed (all accepted)
- `PATCH /contracts/1/status` with invalid status → correctly rejected (HTTP 400)
- `GET /stats` → contracts=1, clients=1
- `POST /scrape` (upwork) → returned `errors: [upwork / rust] status 410 Gone` without crashing (graceful source-level error handling confirmed)

## Completed: Job-hunt automation pipeline (freelance + full-time + outreach)

Turned LeadGen into a full job-hunt pipeline in one dashboard:

### Backend (`backend/src/hunt.rs`, `db.rs`, `api.rs`, `models.rs`)
- `Profile`: saved as `settings.profile.*`; used to score leads and personalise outreach. `score_lead_against_profile` → 0–100 fit (skill overlap + remote/location + recency).
- `generate_outreach` → 3 template-based drafts (freelance proposal, LinkedIn message, email) filled from your profile.
- LinkedIn OAuth: `linkedin_auth_url` (with state), `exchange_linkedin_code`, `linkedin_me`, `connect_linkedin` (stores token/expiry/member id). Official API only — no password, no scraping.
- `applications` table + CRUD: add/list/update/delete; auto-stamps stage timestamps, increments follow-up counts, syncs the source lead status, dedupes by lead_id.
- Routes: `/leads/import` (URL), `/leads/rescore`, `/leads/:id/outreach`, `/applications` CRUD, `/profile` GET/PUT, `/settings/linkedin` GET/PUT, `/linkedin/auth-url|callback|status`.

### Frontend
- `Applications.tsx` (new page + route + nav): pipeline cards with a saved→applied→replied→interviewed→offered→hired stage stepper, reject/close actions, follow-up logging, and an edit modal (company/contact/next/notes).
- `Settings.tsx`: added **My profile** section (name/title/email/location/rate/skills/experience/availability/bio/portfolio/linkedin/github) + **LinkedIn connection** section (app config + "Continue with LinkedIn" via `linkedinAuthUrl`).
- `Leads.tsx`: added **Import URL** modal, **Generate outreach** modal (copy-paste drafts), per-row **Track application** action, plus an Import URL header button.
- `Badge.tsx`: added tone mappings for indeed/linkedin/facebook/saved/replied/interviewed/offered/hired/rejected/closed.
- `types.ts`/`api.ts`: new types (Profile, OutreachDraft, Application/New/Cached) and methods for import/rescore/outreach/applications/profile/linkedin.

### Docs
- Added root `SETUP.md` (LinkedIn OAuth app creation, Indeed notes, running, tests).
- Updated `README.md` overview + endpoint table + job-hunt pipeline section.

### Verification
- `cargo test`: **27/27** (Indeed parse/detection, hunt scoring/profile/outreach/LinkedIn URL, applications).
- `npm run build` (typecheck + vite): green. `npx vitest run`: **19/19**.

## Remaining gaps (future work)

- No Sepolia deployment yet (needs `.env` — blocked on credentials)
- Live LinkedIn OAuth / Indeed RSS fetch can't be verified in an offline sandbox (no network/credentials). The flows are verified by tests + typecheck/build; a real LinkedIn app (see `SETUP.md`) is required live.
- Upwork/Fiverr/Freelancer bot-block scrapers; the app degrades to source errors and users should use **Import URL** / manual leads for those (already supported).
- LinkedIn OAuth redirect callback is designed to return to `:5173/linkedin/callback`; the frontend callback handler now exchanges the code and redirects to Settings.
- **SQLite persistence on Render free tier**: free web services have an ephemeral filesystem — data resets on redeploy. Documented in `DEPLOY.md`; attach a Render Disk (paid) or migrate to Postgres for persistence.

## Completed: Deploy prep (Render + Sepolia path)

Machine-readable deploy config is now in place; the only remaining steps require the user's GitHub/Render/wallet accounts (not available in the sandbox).

- **`render.yaml`** (Render Blueprint): `leadgen-api` web service (`runtime: docker`, `rootDir: backend`, health check `/health`, PORT/BIND/DATABASE_PATH/CORS_ORIGIN/LinkedIn env vars) + `leadgen-web` static site (`rootDir: frontend`, `npm ci && npm run build`, `dist/`, `VITE_API_URL`).
- **`backend/Dockerfile`**: multi-stage Rust 1.97 → debian-slim runtime, `ca-certificates`, `DATABASE_PATH=/app/data/leadgen.db`, exposes 8080. `backend/.dockerignore` excludes `target/` + db files.
- **Backend `main.rs`**: reads `PORT` (Render convention) with `BIND` fallback; CORS restricted via optional comma-separated `CORS_ORIGIN` env (defaults to allow-any for local dev). `cargo test` still 27/27.
- **Frontend**: production build verified with `VITE_API_URL` (URL confirmed embedded in bundle); `npm run build` + `npx vitest run` 19/19 green.
- **`DEPLOY.md`**: full walkthrough — Render Blueprint steps, env vars to set, SQLite persistence caveat, Sepolia contract deploy (`npm run deploy:sepolia`), and free domain options (onrender subdomain / free TLD + CNAME).
- **Contracts**: hardhat `sepolia` network + `deploy:sepolia` script already present; compile verified. Deploy needs user `.env` (SEPOLIA_RPC_URL + PRIVATE_KEY + funded wallet).

## How to run

```bash
# Backend
cd backend && cargo run

# Frontend
cd frontend && npm install && npm run dev

# Contracts
cd contracts && npm install && npm run test
```
