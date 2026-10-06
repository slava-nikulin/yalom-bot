set dotenv-load

backend_image := "yalom-bot"
miniapp_image := "yalom-bot-miniapp"

dev_tag := backend_image + ":dev"
miniapp_dev_tag := miniapp_image + ":dev"

prod_platform := "linux/arm64"

dev_db_url := "mysql://root:root@127.0.0.1:3306/yalom"
test_db_url := "mysql://root:root@127.0.0.1:3307/yalom_test"

# list available commands.
default:
    @just --list

# -------------------------------------------------------------------
# Dev
# -------------------------------------------------------------------

dev-up:
    docker compose -f local/compose.yml up -d --build --wait
    just migrate

dev-db-exec:
    docker compose -f local/compose.yml exec db mysql -uroot -proot yalom

dev-down:
    docker compose -f local/compose.yml down

dev-down-clean:
    docker compose -f local/compose.yml down -v --remove-orphans

tunnel:
    cloudflared tunnel --url http://localhost:8080

# Configure Telegram webhook and Mini App menu button.
telegram-setup:
    #!/usr/bin/env bash
    set -euo pipefail
    set -a
    source local/.env
    set +a

    cargo run --bin telegram_admin

# -------------------------------------------------------------------
# Telegram
# -------------------------------------------------------------------

# Show current Telegram webhook information.
webhook-info:
    curl -fsS "https://api.telegram.org/bot${YALOM_TELEGRAM__TOKEN}/getWebhookInfo" | jq

# -------------------------------------------------------------------
# Rust
# -------------------------------------------------------------------

fmt:
    cargo fmt --all --check

clippy:
    cargo clippy --locked --all-targets --all-features -- -D warnings

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
# Database
# -------------------------------------------------------------------

db-up:
    docker compose -f local/compose.yml up -d db

db-down:
    docker compose -f local/compose.yml down

migration name:
    sqlx migrate add -r {{name}}

_db-reset url:
    DATABASE_URL='{{url}}' sqlx database reset -y

_migrate url:
    DATABASE_URL='{{url}}' sqlx migrate run

_migrate-down url:
    DATABASE_URL='{{url}}' sqlx migrate revert

_migrate-down-all url:
    DATABASE_URL='{{url}}' sqlx migrate revert --target-version 0

db-reset: (_db-reset dev_db_url)

migrate: (_migrate dev_db_url)

migrate-down: (_migrate-down dev_db_url)

migrate-down-all: (_migrate-down-all dev_db_url)

# -------------------------------------------------------------------
# Test
# -------------------------------------------------------------------

test-db-up:
    MYSQL_HOST_PORT=3307 \
    MYSQL_DATABASE=yalom_test \
    docker compose -p yalom-test -f local/compose.yml up -d --wait db

test-db-down:
    docker compose -p yalom-test -f local/compose.yml down -v --remove-orphans

test:
    #!/usr/bin/env bash
    set -euo pipefail

    trap 'docker compose -p yalom-test -f local/compose.yml down -v --remove-orphans' EXIT

    if ! MYSQL_HOST_PORT=3307 MYSQL_DATABASE=yalom_test \
        docker compose -p yalom-test -f local/compose.yml up -d --wait db
    then
        docker compose -p yalom-test -f local/compose.yml logs --no-color db
        exit 1
    fi

    DATABASE_URL='{{test_db_url}}' cargo test

migrations-check: (_db-reset test_db_url) (_migrate-down-all test_db_url) (_migrate test_db_url)

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
check-full: frontend-install check test frontend-build docker-check-all