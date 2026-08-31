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
- Scrape endpoint with Upwork RSS, Freelancer AJAX, Fiverr regex (fragile)
- Deploy endpoint records tx_hash + contract_address
- `PATCH /contracts/:id/status` — lifecycle status updates (deployed/funded/in_progress/submitted/completed/disputed/refunded)

### frontend/
- React + Vite + Tailwind on `:5173`
- Pages: Dashboard, Leads, Clients, Contracts, Settings
- Wallet integration (MetaMask via ethers v6)
- Deploy escrow from wallet (one-shot)
- **Escrow lifecycle UX (done)**: Contracts page shows on-chain state (via `getEscrowInfo`), role-based action buttons (Start work / Submit work / Approve & pay / Raise dispute / Cancel & refund / Refund after deadline / Resolve dispute with 50/50 split), on-chain info panel (state, deposit, deadline, mediator)

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

## Remaining gaps (future work)

- No Sepolia deployment yet (needs `.env` — blocked on credentials)
- Live scraping of job sites is limited by anti-bot measures; runs degrade to source errors. A production deployment should use authenticated sessions, a headless browser, or third-party job APIs/actors for reliable ingestion.

## How to run

```bash
# Backend
cd backend && cargo run

# Frontend
cd frontend && npm install && npm run dev

# Contracts
cd contracts && npm install && npm run test
```
