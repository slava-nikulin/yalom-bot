set dotenv-load

image := "yalom-bot"
dev_tag := image + ":dev"
prod_platform := "linux/arm64"


# Показать доступные команды.
default:
    @just --list

# Start a Cloudflare Quick Tunnel for the local HTTP server.
tunnel:
    cloudflared tunnel --url http://localhost:3000

# Show current Telegram webhook information.
webhook-info:
    curl -fsS "https://api.telegram.org/bot${YALOM_TELEGRAM__TOKEN}/getWebhookInfo" | jq

# Set webhook url and webhook secret for the bot
telegram-set-webhook:
    cargo run --bin telegram_admin --no-default-features


# -------------------------------------------------------------------
# Rust
# -------------------------------------------------------------------

fmt:
    cargo fmt --all --check

clippy:
    cargo clippy --locked --all-targets --all-features -- -D warnings

test:
    cargo test --locked --no-default-features

test-all:
    cargo test --locked --all-features


# -------------------------------------------------------------------
# Docker
# -------------------------------------------------------------------

# Собрать образ под текущую архитектуру.
docker-build:
    docker buildx build \
        --load \
        --tag {{dev_tag}} \
        .

# Собрать production ARM64 image.
docker-build-arm64:
    docker buildx build \
        --platform {{prod_platform}} \
        --load \
        --tag {{image}}:arm64 \
        .

# Проверить сборку обеих поддерживаемых архитектур.
docker-check:
    docker buildx build \
        --platform linux/amd64,linux/arm64 \
        .

# Запустить локально.
docker-run: docker-build
    docker run \
        --rm \
        --init \
        --env-file .env \
        --publish 3000:3000 \
        {{dev_tag}}


# -------------------------------------------------------------------
# Checks
# -------------------------------------------------------------------

check: fmt clippy test docker-build