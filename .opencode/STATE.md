# LeadGen — Project State

> Last updated: 2026-08-31

## Git status

- Branch: `main`, clean tree, pushed to `origin/main`
- Remote: `Pratikeeyyyy/Work`
- 5 commits: contracts → backend → frontend → 2 merges

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
- **Missing**: contract lifecycle status tracking beyond "deployed"

### frontend/
- React + Vite + Tailwind on `:5173`
- Pages: Dashboard, Leads, Clients, Contracts, Settings
- Wallet integration (MetaMask via ethers v6)
- Deploy escrow from wallet (one-shot)
- **Missing**: escrow lifecycle actions (start/submit/approve/dispute/refund)

## Current task: Escrow lifecycle UX

### What needs to be built

1. **Backend** — `PATCH /contracts/:id/status` endpoint + `update_contract_status` in db.rs
2. **Frontend lib/escrow.ts** — wallet functions for: `startWork`, `submitWork`, `approve`, `cancelBeforeWork`, `raiseDispute`, `resolveDispute`, `refundAfterDeadline`, `getEscrowInfo`
3. **Frontend api.ts** — `updateContractStatus(id, status)` method
4. **Frontend Badge.tsx** — add `in_progress`, `submitted` tones
5. **Frontend Contracts.tsx** — show on-chain state, action buttons based on role (client/freelancer/mediator) and current state
6. **Backend db.rs** — `update_contract_status(id, status)` method

### Contract state machine (for reference)

```
Funded(0) → startWork() → InProgress(1) → submitWork() → Submitted(2)
Submitted(2) → approve() → Completed(3)
InProgress(1) | Submitted(2) → raiseDispute() → Disputed(4)
Disputed(4) → resolveDispute(share) → Completed(3)
Funded(0) → cancelBeforeWork() → Refunded(5)
After deadline → refundAfterDeadline() → Refunded(5)
```

### Files to modify

- `backend/src/db.rs` — add `update_contract_status()`
- `backend/src/api.rs` — add route + handler for `PATCH /contracts/:id/status`
- `frontend/src/lib/escrow.ts` — add all escrow action functions
- `frontend/src/api.ts` — add `updateContractStatus()`
- `frontend/src/components/Badge.tsx` — add `in_progress`, `submitted` statuses
- `frontend/src/pages/Contracts.tsx` — lifecycle actions UI

## Other known gaps (future work)

- Fiverr scraper `__NEXT_DATA__` parser is a placeholder
- No Sepolia deployment yet
- No backend or frontend tests
- No README for individual components

## How to run

```bash
# Backend
cd backend && cargo run

# Frontend
cd frontend && npm install && npm run dev

# Contracts
cd contracts && npm install && npm run test
```
