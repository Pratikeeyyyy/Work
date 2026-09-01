# SETUP

This app is built to be safe and legal: no account/password scraping, no ToS-violating scrapers. It connects to job sources through official APIs (LinkedIn OAuth) and public feeds (Indeed RSS), plus manual import.

## Run the app

```bash
# Backend (Rust + Axum) — port 8080
cd backend
cargo run

# Frontend (React + Vite) — port 5173
cd frontend
npm install
npm run dev
```

Open http://localhost:5173.

## Multi-user authentication

LeadGen supports multiple accounts. Each account has its **own isolated data** (leads, clients, contracts, applications, profile, settings) stored in a separate database file, so users never see each other's data. On first launch you'll see the login screen — switch to **Create an account**, pick a username + password, and you're in.

Passwords are **never stored in plaintext** — they're hashed with PBKDF2-HMAC-SHA256 (random salt). Tokens are opaque random strings held in memory (7-day TTL) and invalidated on server restart.

What changed for existing installs:

- The old single-user `APP_PASSWORD`/first-run `Setup` flow is replaced by accounts. Register at **Create an account**, then log in.
- Each user's data lives in a file named `user_<username>.db` next to the main database (`leadgen.db`, which now only holds the account registry). Data created before this change in the old single-user database does **not** automatically carry into an account.

> If you forget a password, there is no account-recovery bypass. Registered accounts are listed in the `users` table of the main database.

## Sources and why

| Source | Method | Notes |
|--------|--------|-------|
| **Indeed** | Public RSS feed | Reliable for full-time/remote jobs. Built from your keywords. |
| **Upwork** | Manual import | Upwork disabled RSS in 2024 (returned 410) and bot-blocks scrapers. Use **Import URL**. |
| **Fiverr / Freelancer** | Manual import | Both bot-block scrapers. Use **Import URL**. |
| **LinkedIn** | Official OAuth app | Legal. Never uses your password. Requires a free LinkedIn developer app (below). |
| **Facebook** | Manual import | Paste group post links. |

The **Import URL** button on the Leads page and `POST /leads/import` let you save any job/gig/link as a lead, then score it and generate outreach against your profile.

## Connect LinkedIn (official OAuth)

1. Go to [LinkedIn Developer Portal](https://www.linkedin.com/developers/apps) → **Create app**.
2. Add the **Sign In with LinkedIn** product (and **Share on LinkedIn** if you want posting later).
3. Under **Auth → Redirect URLs**, add your app's redirect URI, e.g.:
   - `http://localhost:5173/linkedin/callback` (Vite dev)
4. Copy the **Client ID** and **Client Secret**.
5. In the app: **Settings → LinkedIn connection**, paste both, keep the redirect URI, and **Save app settings**.
6. Click **Continue with LinkedIn** — a popup opens the official authorize screen. After you approve, LinkedIn redirects with a code; paste the callback result if auto-exchange doesn't complete, or confirm via **LinkedIn connection** status.

Your Client Secret is stored in SQLite and never logged. The app only calls official LinkedIn APIs with your token — it does not scrape profiles.

> Keep the Client Secret out of the git repo. Do not commit `.env` or the database with real tokens to GitHub.

## Profile for scoring & outreach

Fill **Settings → My profile**:
- **Skills** (comma-separated) drive the fit score (higher overlap = higher lead score, 0–100).
- **Location / Remote** and **Availability** affect location-fit.
- **Name, Email, LinkedIn, GitHub, Portfolio, Bio** are inserted into auto-drafted outreach (proposal / LinkedIn message / email).

After editing your profile, run **Rescore** on the Leads page to re-rank all leads.

## Application tracking

On any lead, click **Track application** to add it to the Applications page, then move it through:
`saved → applied → replied → interviewed → offered → hired` — with follow-up logging, next-step scheduling, and notes.

## Discover & auto-apply

- **Discover → auto-discovery**: toggle it on and the backend scans Indeed on a schedule (default every 30 min), scores new jobs against your profile, and queues anything scoring at/above the threshold. "Scan now" runs an immediate pull. Source = public Indeed RSS only (legal).
- **Apply kit**: each queued lead's **Apply** button builds your proposal, LinkedIn message, and email from your profile, plus the real job link — a review-and-confirm flow you submit yourself.
- **Follow-up due**: lists applications that have gone quiet so you know whom to chase today.

## Tests

```bash
cd backend && cargo test        # 45 tests
cd frontend && npx vitest run   # 19 tests
cd contracts && npm run test    # escrow
```
