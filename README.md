# LeadGen

All-in-one job-hunt automation: freelance gigs + full-time jobs + client/lead outreach in one dashboard, with lead fit-scoring, auto-drafted outreach, application tracking, and escrow-backed contracts. Protected by official legal sources only — Indeed RSS, LinkedIn OAuth, and manual import — never account/password scraping.

## Architecture

```
contracts/   Solidity + Hardhat     FreelanceEscrow.sol — on-chain escrow
backend/     Rust + Axum            REST API, SQLite, multi-source scrapers, scoring + outreach
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

## Auth

The app is single-user. All business endpoints require a bearer token (`Authorization: Bearer <token>`). The first time you run the server you either set `APP_PASSWORD` (recommended) or create a password once via `/auth/setup`; then log in to get a token (7-day session). See `SETUP.md`.

## API endpoints

Public (no auth):

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Health check |
| GET | `/auth/status` | Auth status (`hasPassword`, `authenticated`) |
| POST | `/auth/setup` | Create the single-user password (simplified setup) |
| POST | `/login` | Log in with password → returns session token |
| POST | `/auth/logout` | Invalidate current session |

Protected (require `Authorization: Bearer <token>`):

| Method | Path | Description |
|--------|------|-------------|
| GET | `/stats` | Dashboard stats |
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
| POST | `/leads/import` | Import a job/gig from a pasted URL |
| POST | `/leads/rescore` | Re-score all leads against your profile |
| GET | `/leads/:id/outreach` | Generate auto-drafted outreach (proposal / LinkedIn / email) |
| GET/PUT | `/profile` | Your profile used for scoring + outreach |
| GET/POST | `/applications` | List / add tracked applications |
| PATCH/DELETE | `/applications/:id` | Update / delete an application |
| GET/PUT | `/settings/linkedin` | LinkedIn OAuth app config |
| GET | `/linkedin/auth-url` | Build LinkedIn OAuth authorize URL |
| POST | `/linkedin/callback` | Exchange LinkedIn auth code |
| GET | `/linkedin/status` | LinkedIn connection status |

## Job-hunt pipeline

1. **Gather** — run a scrape (Indeed RSS is reliable; Upwork/Fiverr/Freelancer block bots, so use import or manual) or paste any job/URL into **Import URL**.
2. **Score** — fill **Settings → My profile** (skills, location, rate) and hit **Rescore** on the Leads page. Leads are ranked 0–100 by skill overlap + location + recency.
3. **Outreach** — open any lead and **Generate outreach** to get a copy-paste proposal, LinkedIn message, and email drafted from your profile.
4. **Track** — on any lead, **Track application** promotes it into the Applications page where you move it through saved → applied → replied → interviewed → offered → hired, with follow-up nudges and notes.
5. **Close** — convert won leads to clients and escrow contracts protect payments.

See `SETUP.md` for LinkedIn OAuth app creation and source notes.

## Tech stack

- **Backend**: Rust, Axum 0.7, rusqlite (bundled SQLite), reqwest, rss
- **Contracts**: Solidity ^0.8.20, Hardhat, ethers v6
- **Frontend**: React 18, TypeScript, Vite 6, Tailwind CSS v4, ethers v6, react-router-dom v6
