## Yalom bot mini app

Solid 2 RC
TypeScript
TSRX
Vite 8
TanStack Router 2 RC
TanStack Query 6 RC
UnoCSS + Wind4
Oxlint
Vitest

## Run with the Rust backend

From the repository root:

```sh
cd web/miniapp
pnpm install --frozen-lockfile
pnpm build
cd ../..
test -f .env || cp .env.example .env
```

Fill in `.env` with a Telegram bot token, webhook secret token, webhook URL, and Mini App session key. The webhook URL must be a valid URL even when automatic webhook registration is disabled. Then run:

```sh
set -a
source .env
set +a
cargo run --no-default-features --bin yalom-bot
```

Open `http://localhost:3000/miniapp` to check that the SPA and its assets are served. The app can complete session bootstrap only when Telegram opens it and supplies `Telegram.WebApp.initData`. For that flow, expose port 3000 through HTTPS (for example, `just tunnel`) and open the tunnel's `/miniapp` URL from Telegram. The frontend posts to `/miniapp/session` on the same origin.
