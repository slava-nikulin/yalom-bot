# syntax=docker/dockerfile:1

FROM --platform=$BUILDPLATFORM tonistiigi/xx:1.8.0 AS xx

FROM --platform=$BUILDPLATFORM rust:1.98.0-alpine3.24 AS build
COPY --from=xx / /

ARG TARGETPLATFORM
ARG TARGETARCH

RUN apk add --no-cache \
        ca-certificates \
        clang \
        lld \
    && xx-apk add --no-cache \
        gcc \
        musl-dev

WORKDIR /src

RUN --mount=type=bind,source=.,target=/src,readonly \
    --mount=type=cache,id=yalom-cargo-registry,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=yalom-cargo-git,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=yalom-target-${TARGETARCH},target=/build,sharing=locked \
    xx-cargo build \
        --locked \
        --release \
        --no-default-features \
        --bins \
        --target-dir /build \
    && target="$(xx-cargo --print-target-triple)" \
    && cp "/build/${target}/release/yalom-bot" /yalom-bot \
    && cp "/build/${target}/release/telegram_admin" /telegram_admin \
    && xx-verify --static /yalom-bot \
    && xx-verify --static /telegram_admin

FROM scratch AS final

LABEL org.opencontainers.image.title="yalom-bot" \
      org.opencontainers.image.description="Ambient Telegram bot powered by LLM" \
      org.opencontainers.image.source="https://github.com/slava-nikulin/yalom-bot" \
      org.opencontainers.image.licenses="MIT"

COPY --from=build \
    /etc/ssl/certs/ca-certificates.crt \
    /etc/ssl/certs/ca-certificates.crt

COPY --from=build --chmod=0555 \
    /yalom-bot \
    /telegram_admin \
    /usr/local/bin/

USER 10001:10001

EXPOSE 3000
STOPSIGNAL SIGTERM

ENTRYPOINT ["/usr/local/bin/yalom-bot"]