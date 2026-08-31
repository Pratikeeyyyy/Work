# LeadGen Frontend

React dashboard for managing leads, clients, and on-chain escrow contracts.

## Stack

- **React 18** + **TypeScript** (strict)
- **Vite 6**
- **Tailwind CSS v4**
- **ethers v6** (wallet + escrow)
- **react-router-dom** v6

## Run

```bash
npm install
npm run dev      # http://localhost:5173, proxies API to :8080
```

## Scripts

| Script | Description |
|--------|-------------|
| `npm run dev` | Dev server with API proxy |
| `npm run build` | Typecheck + production build |
| `npm run typecheck` | TypeScript only |
| `npm run test` | Run unit/component tests (Vitest) |
| `npm run test:watch` | Run tests in watch mode |
| `npm run preview` | Preview production build |

## Environment

| Variable | Default | Description |
|----------|---------|-------------|
| `VITE_API_URL` | `""` (proxy) | Backend base URL |

## Pages

- **Dashboard** — stats cards, charts, scrape trigger
- **Leads** — pipeline table with filters, notes, convert-to-client
- **Clients** — client CRM table
- **Contracts** — escrow lifecycle: deploy, start/submit/approve work, disputes, refunds
- **Settings** — scrape keywords + enabled sources

## Key files

- `src/lib/wallet.tsx` — MetaMask wallet context
- `src/lib/escrow.ts` — escrow deploy + lifecycle wrappers, `explorerUrl`
- `src/lib/FreelanceEscrow.json` — compiled contract ABI/bytecode
- `src/api.ts` — typed REST client
- `src/pages/Contracts.tsx` — full escrow lifecycle UI

## Wallet / escrow flow

1. Connect MetaMask (requires a testnet/fork with ETH)
2. Create a contract against a client with a freelancer address + amount
3. **Deploy escrow** — client deposits the amount
4. Freelancer: **Start work** → **Submit work**
5. Client: **Approve & pay** (or **Raise dispute**)
6. Mediator: **Resolve dispute** (split funds) if a dispute is raised
