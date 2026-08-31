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

## Remaining gaps (future work)

- No Sepolia deployment yet (needs `.env` — blocked on credentials)

## How to run

```bash
# Backend
cd backend && cargo run

# Frontend
cd frontend && npm install && npm run dev

# Contracts
cd contracts && npm install && npm run test
```
