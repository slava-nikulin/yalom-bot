use rustigram_api::BotClient;
use sqlx::mysql::MySqlPoolOptions;
use std::{net::SocketAddr, sync::Arc};
use tokio::signal::unix::{SignalKind, signal};
use tracing_subscriber::EnvFilter;
use yalom_bot::{
    app_state::AppState,
    config::AppConfig,
    http::app_router,
    nudge::{
        NudgeService, open_ai_adapter::OpenAiAdapter, rate_limit::ManualNudgeMySqlLimiter,
        store::NudgeMySqlStore,
    },
    security::session::Sessions,
    telegram::sender::TgSenderService,
    user::store::MySqlUserStore,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = Arc::new(AppConfig::load()?);
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn,yalom_bot=info"));

    tracing_subscriber::fmt()
        .json()
        .flatten_event(true)
        .with_ansi(false)
        .with_env_filter(filter)
        .init();

    let tg_webhook_secret_token = &cfg.telegram.webhook_secret;

    let tg_bot = BotClient::from_token(&cfg.telegram.token)?;

    let db = MySqlPoolOptions::new().connect(&cfg.database.url).await?;
    let user_store = MySqlUserStore::new(db.clone());
    let nudge_store = NudgeMySqlStore::new(db.clone());
    let nudge_limiter = ManualNudgeMySqlLimiter::new(db);

    let open_ai_adapter = OpenAiAdapter::new(cfg.openai.api_key.clone());
    let sessions = Sessions::new(&cfg.telegram.miniapp_session_key);

    let (tg_update_service, telegram_update_pool) =
        TgSenderService::new(tg_bot.clone(), user_store.clone(), 2, 10);

    let nudge_service = NudgeService::new(open_ai_adapter, nudge_store);

    let app_state = AppState::new(
        &cfg.telegram.token,
        tg_webhook_secret_token.as_str(),
        sessions,
        tg_bot,
        user_store,
        tg_update_service,
        nudge_service,
        nudge_limiter,
    );
    let app_router = app_router(app_state);

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

    telegram_update_pool.shutdown().await;

    tracing::info!("server shut down gracefully");

    Ok(())
}
