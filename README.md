# LeadGen

Freelance lead-generation and escrow-management platform. Scrapes gig boards, manages a client pipeline, and protects payments with on-chain ETH escrow.

## Architecture

```
contracts/   Solidity + Hardhat     FreelanceEscrow.sol — on-chain escrow
backend/     Rust + Axum            REST API, SQLite, multi-source scrapers
frontend/    React + Vite + Tailwind  Dashboard, pipeline, wallet integration
```

### Escrow lifecycle

```
Deploy (client deposits ETH) → Funded
  ↳ Freelancer: startWork() → InProgress
    ↳ Freelancer: submitWork() → Submitted
      ↳ Client: approve() → Completed (funds released)
      ↳ Either: raiseDispute() → Disputed → mediator resolves
  ↳ Client: cancelBeforeWork() → Refunded
  ↳ Client: refundAfterDeadline() → Refunded (after 30 days)
```

## Getting started

### Backend

```bash
cd backend
cargo run          # starts on :8080, creates leadgen.db
```

### Frontend

```bash
cd frontend
npm install
npm run dev        # starts on :5173, proxies API to :8080
```

### Contracts

```bash
cd contracts
npm install
npm run compile    # compile Solidity
npm run test       # run escrow tests
npm run deploy:local   # deploy to local Hardhat node
```

For Sepolia/mainnet deploy, copy `.env.example` → `.env` and fill in keys.

## API endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Health check |
| GET | `/stats` | Dashboard stats |
| GET/POST | `/leads` | List / create leads |
| GET/DELETE | `/leads/:id` | Get / delete a lead |
| PATCH | `/leads/:id/status` | Update lead status |
| PATCH | `/leads/:id/notes` | Update lead notes |
| POST | `/leads/:id/to-client` | Convert lead to client |
| GET/POST | `/clients` | List / create clients |
| GET/PUT/DELETE | `/clients/:id` | CRUD a client |
| GET/POST | `/contracts` | List / create contracts |
| POST | `/contracts/:id/deploy` | Record on-chain deployment |
| PATCH | `/contracts/:id/status` | Update contract lifecycle status |
| GET/PUT | `/settings/keywords` | Manage scrape keywords |
| GET/PUT | `/settings/sources` | Manage enabled sources |
| POST | `/scrape` | Run scrape now |

## Tech stack

- **Backend**: Rust, Axum 0.7, rusqlite (bundled SQLite), reqwest, rss
- **Contracts**: Solidity ^0.8.20, Hardhat, ethers v6
- **Frontend**: React 18, TypeScript, Vite 6, Tailwind CSS v4, ethers v6, react-router-dom v6
