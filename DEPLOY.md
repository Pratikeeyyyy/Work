# Deployment Guide

This project deploys to free hosting with Render, plus a testnet contract, and (optional) a free
custom domain. Everything here is driven from GitHub (`github.com/Pratikeeyyyy/Work`) and the
Render dashboard — there is no need to run anything locally for the web app.

The repo ships an **automatic Render Blueprint** (`render.yaml`). Once you connect the GitHub repo
to Render, it provisions both services for you. You only fill in a few environment variables by hand.

---

## 1. Deploy the web app (frontend + backend) to Render — free

Render free tier runs everything from a `blueprint` in `render.yaml`:

- `leadgen-api` — the Rust/Axum backend, built with `backend/Dockerfile`, served as a web service.
- `leadgen-web` — the React frontend, built with `npm run build`, served as a static site.

### Steps

1. Push this repo to GitHub (`git push origin main`) — already done.
2. Sign up / log in at [render.com](https://render.com) with GitHub.
3. Click **New → Blueprint** → connect the **Pratikeeyyyy/Work** repo.
4. Render reads `render.yaml` and proposes the two services. Click **Apply** (free plan).
5. After it provisions, open **leadgen-api** and **leadgen-web** in the dashboard.

### Environment variables to set

Fill these in the Render dashboard under each service's **Environment** tab, then **Deploy**:

**leadgen-api**
| Var | Value | Notes |
|-----|-------|-------|
| `CORS_ORIGIN` | `https://leadgen-web.onrender.com` | Comma-separated allowed frontend origin(s). |
| `LINKEDIN_CLIENT_ID` | (id) | Optional — from your LinkedIn developer app (see `SETUP.md`). |
| `LINKEDIN_CLIENT_SECRET` | (secret) | Optional — never commit this. |
| `DATABASE_PATH` | `/app/data/leadgen.db` | Already defaulted in the Dockerfile. |

**leadgen-web**
| Var | Value | Notes |
|-----|-------|-------|
| `VITE_API_URL` | `https://leadgen-api.onrender.com` | API base URL baked into the frontend at build time. |

> The exact backend URL shows in the dashboard (e.g. `https://leadgen-api.onrender.com`). Use that
> for `VITE_API_URL` and `CORS_ORIGIN`.

Your public app will be at `https://leadgen-web.onrender.com` (the exact subdomain shown in the
dashboard).

### Important: SQLite persistence on free tier

Render free web services have an **ephemeral filesystem** — the SQLite database resets every time
the service restarts or redeploys. That's fine for iterating and testing (your config comes back
via the blueprint), but not for production data.

To keep data across deploys you have two options (both invoiced by Render, not truly free):
- Attach a **Render Disk** to `leadgen-api` mounted at `/app/data` (small disks are cheap), or
- Run on a paid plan, or
- Move storage to a managed Postgres later (a bigger refactor).

---

## 2. Deploy the smart contract to Sepolia (testnet)

The contract deploy path is already wired (`hardhat.config.js` has a `sepolia` network and
`contracts/scripts/deploy.js` exists).

From `contracts/`:

```bash
cp .env.example .env
# edit .env — fill in:
#   SEPOLIA_RPC_URL  (from Infura/Alchemy/your node)
#   PRIVATE_KEY      (the deploying account's private key — NEVER commit this)
#   FREELANCER_ADDRESS / MEDIATOR_ADDRESS (optional)
#   AMOUNT_ETH        (default 0.1)

# The address needs testnet ETH. Get some from a Sepolia faucet.

npm install
npm run deploy:sepolia
```

Copy the printed `FreelanceEscrow deployed to:` address into the app's Contracts page when you
record a deploy. (A deployed address is not "baked in" anywhere in the code — you paste it per
contract, which keeps the flow flexible.)

Verify on Etherscan with `npx hardhat verify --network sepolia <ADDRESS> <freelancer> <mediator>`
(requires `ETHERSCAN_API_KEY`).

---

## 3. Free custom domain

Options, cheapest first. The app itself doesn't care — both services are served over HTTPS, so any
of these work.

1. **Free subdomain (already have one)** — `leadgen-web.onrender.com` is free and needs zero setup.
2. **Free domain from a provider** — e.g. get a free domain (like a `.pp.ua` / `.com.np` / `.tk`)
   and add it as a custom domain in Render:
   - **leadgen-web** (static site): Settings → **Static Site → Custom Domains** → add domain →
     point a `CNAME` (or Render's `_render` TXT for the apex) at `leadgen-web.onrender.com`.
   - **leadgen-api** (web service): same, CNAME to `leadgen-api.onrender.com`, and append the domain
     to `CORS_ORIGIN` on the API side.
3. **Free SSL** — Render issues Let's Encrypt certs automatically for custom domains.

If you don't want to buy a domain, staying on the default `*.onrender.com` subdomains is the simplest
and fully free path.

---

## What the app does on a real deployment

With the frontend and API live, the full job-hunt pipeline works over the internet:

- **Scrape / import**: Indeed RSS (reliable) + manual Import-URL for Upwork/Fiverr/others.
- **Score**: fill your profile in Settings → run Rescore — leads ranked 0–100 by fit.
- **Outreach**: auto-drafted proposal / LinkedIn message / email per lead.
- **Applications**: track saved → applied → … → hired with follow-ups.
- **Escrow**: record sepolia contract addresses on the Contracts page.

---

## Verify after deploy

- `https://<api>.onrender.com/health` → should return OK.
- `GET https://<api>.onrender.com/stats` → JSON stats.
- Open the frontend, add a lead, run a scrape, check the Applications page.
