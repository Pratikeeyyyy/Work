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
