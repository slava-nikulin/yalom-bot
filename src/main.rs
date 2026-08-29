use rustigram_api::BotClient;
use std::{net::SocketAddr, sync::Arc};
use tokio::signal::unix::{SignalKind, signal};
use tracing_subscriber::EnvFilter;
use yalom_bot::{
    app_state::AppState, config::Config, http::app_router, security::generate_secret_token,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn,yalom_bot=info"));

    tracing_subscriber::fmt()
        .json()
        .flatten_event(true)
        .with_ansi(false)
        .with_env_filter(filter)
        .init();

    let cfg = Arc::new(Config::load()?);

    let tg_webhook_secret_token = match &cfg.telegram.webhook.secret_token {
        Some(token) => token.clone(),
        None => {
            #[cfg(feature = "local-dev")]
            {
                let token = generate_secret_token(32);
                tracing::info!(token = %token, "tg_webhook_secret_token generated");
                token
            }

            #[cfg(not(feature = "local-dev"))]
            {
                anyhow::bail!("telegram webhook secret token is required");
            }
        }
    };

    let tg_bot = BotClient::from_token(&cfg.telegram.token)?;

    #[cfg(feature = "auto-webhook")]
    {
        tg_bot
            .set_webhook(cfg.telegram.webhook.url.clone())
            .secret_token(&tg_webhook_secret_token)
            .await?;

        tracing::info!(
            url = %cfg.telegram.webhook.url,
            "telegram webhook registered"
        );
    }

    let app_state = AppState::new(tg_bot);
    let app_router = app_router(tg_webhook_secret_token, app_state);

    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;

    let shutdown_signal = async move {
        tokio::select! {
            _ = sigterm.recv() => {
                tracing::info!("received SIGTERM");
            }
            _ = sigint.recv() => {
                tracing::info!("received SIGINT");
            }
        }
    };

    let addr = SocketAddr::new(cfg.http.host, cfg.http.port);
    let tcp_listener = tokio::net::TcpListener::bind(addr)
        .await
        .inspect_err(|err| tracing::error!(error = %err, "failed to bind tcp listener"))?;
    axum::serve(tcp_listener, app_router)
        .with_graceful_shutdown(shutdown_signal)
        .await?;

    tracing::info!("server shut down gracefully");

    Ok(())
}
