set dotenv-load

# Start a Cloudflare Quick Tunnel for the local HTTP server.
tunnel:
    cloudflared tunnel --url http://localhost:3000

# Show current Telegram webhook information.
webhook-info:
    curl -fsS "https://api.telegram.org/bot${YALOM_TELEGRAM__TOKEN}/getWebhookInfo" | jq

# Set webhook url and webhook secret for the bot
telegram-set-webhook:
    cargo run --bin telegram_admin --no-default-features