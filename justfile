set dotenv-load

backend_image := "yalom-bot"
miniapp_image := "yalom-miniapp"

dev_tag := backend_image + ":dev"
miniapp_dev_tag := miniapp_image + ":dev"

prod_platform := "linux/arm64"


# list available commands.
default:
    @just --list

# Start a Cloudflare Quick Tunnel for the local HTTP server.
tunnel:
    cloudflared tunnel --url http://localhost:3000

# Show current Telegram webhook information.
webhook-info:
    curl -fsS "https://api.telegram.org/bot${YALOM_TELEGRAM__TOKEN}/getWebhookInfo" | jq

# Set webhook url and webhook secret for the bot
telegram-setup:
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
# k8s
# -------------------------------------------------------------------

k8s-check:
    kubectl kustomize deploy/k8s/overlays/oci-k3s >/dev/null


# -------------------------------------------------------------------
# Docker
# -------------------------------------------------------------------

# build backend for current architecture.
docker-build:
    docker buildx build \
        --load \
        --tag {{dev_tag}} \
        .

# build miniapp for current architecture.
docker-build-miniapp:
    docker buildx build \
        --file Dockerfile.miniapp \
        --load \
        --tag {{miniapp_dev_tag}} \
        .

# build backend for ARM64 architecture.
docker-build-arm64:
    docker buildx build \
        --platform {{prod_platform}} \
        --load \
        --tag {{backend_image}}:arm64 \
        .

# build miniapp for ARM64 architecture.
docker-build-miniapp-arm64:
    docker buildx build \
        --file Dockerfile.miniapp \
        --platform {{prod_platform}} \
        --load \
        --tag {{miniapp_image}}:arm64 \
        .

# check backend for both archs
docker-check:
    docker buildx build \
        --platform linux/amd64,linux/arm64 \
        .

# check miniapp for both archs
docker-check-miniapp:
    docker buildx build \
        --file Dockerfile.miniapp \
        --platform linux/amd64,linux/arm64 \
        .

# check backend and miniapp
docker-check-all: docker-check docker-check-miniapp

# run backend
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

check: fmt clippy test k8s-check
check-full: check docker-check-all