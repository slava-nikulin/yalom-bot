set dotenv-load

backend_image := "yalom-bot"
miniapp_image := "yalom-bot-miniapp"

dev_tag := backend_image + ":dev"
miniapp_dev_tag := miniapp_image + ":dev"

prod_platform := "linux/arm64"

# list available commands.
default:
    @just --list

# -------------------------------------------------------------------
# Dev
# -------------------------------------------------------------------

dev-backend:
    cargo run --no-default-features --bin yalom-bot

dev-miniapp:
    cd web/miniapp && pnpm dev

tunnel-backend:
    cloudflared tunnel --url http://localhost:3000

tunnel-miniapp:
    cloudflared tunnel \
        --url http://localhost:5173 \
        --http-host-header localhost:5173

# -------------------------------------------------------------------
# Telegram
# -------------------------------------------------------------------

# Show current Telegram webhook information.
webhook-info:
    curl -fsS "https://api.telegram.org/bot${YALOM_TELEGRAM__TOKEN}/getWebhookInfo" | jq

# Configure Telegram webhook and Mini App menu button.
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

production-build:
    cargo build --release --no-default-features --locked

# -------------------------------------------------------------------
# Frontend
# -------------------------------------------------------------------

frontend-install:
    cd web/miniapp && pnpm install --frozen-lockfile

frontend-lint:
    cd web/miniapp && pnpm lint

frontend-typecheck:
    cd web/miniapp && pnpm typecheck

frontend-build:
    cd web/miniapp && pnpm build

frontend-check: frontend-lint frontend-typecheck frontend-build

# -------------------------------------------------------------------
# k8s
# -------------------------------------------------------------------

k8s-check:
    kubectl kustomize deploy/k8s/overlays/oci-k3s >/dev/null


# -------------------------------------------------------------------
# Docker
# -------------------------------------------------------------------

docker-build:
    docker buildx build \
        --load \
        --tag {{dev_tag}} \
        .

docker-build-miniapp:
    docker buildx build \
        --file Dockerfile.miniapp \
        --load \
        --tag {{miniapp_dev_tag}} \
        .

docker-build-arm64:
    docker buildx build \
        --platform {{prod_platform}} \
        --load \
        --tag {{backend_image}}:arm64 \
        .

docker-build-miniapp-arm64:
    docker buildx build \
        --file Dockerfile.miniapp \
        --platform {{prod_platform}} \
        --load \
        --tag {{miniapp_image}}:arm64 \
        .

docker-check:
    docker buildx build \
        --platform linux/amd64,linux/arm64 \
        .

docker-check-miniapp:
    docker buildx build \
        --file Dockerfile.miniapp \
        --platform linux/amd64,linux/arm64 \
        .

docker-check-all: docker-check docker-check-miniapp

# -------------------------------------------------------------------
# Checks
# -------------------------------------------------------------------

# Fast feedback loop.
check: fmt clippy test k8s-check frontend-lint frontend-typecheck

# Everything expected to succeed before merge/release.
check-full: frontend-install check test-all production-build frontend-build docker-check-all