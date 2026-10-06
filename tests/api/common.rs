pub mod helper;
pub mod tma;

use std::sync::{Arc, Mutex};

use axum::Router;
use yalom_bot::{
    app_state::AppState,
    http::app_router,
    security::session::Sessions,
    telegram::{
        TelegramClient,
        update_service::{TgUpdateJob, TgUpdateService},
    },
    user::store::MySqlUserStore,
    worker_pool::WorkerPool,
};

pub const TG_WEBHOOK_SECRET_TOKEN: &str = "tg_webhook_secret_token";
pub const TG_TOKEN: &str = "123456789:ABCdefGhIJKlmNoPQRsTUVwxYZ";
pub const SESSION_KEY: &str = "miniapp_session_key";

pub struct TgCall {}

#[derive(Clone)]
pub struct TestBotClient {
    pub calls: Arc<Mutex<Vec<TgCall>>>,
}

impl TestBotClient {
    fn new() -> Self {
        Self {
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl TelegramClient for TestBotClient {
    async fn send_message(
        &self,
        _chat_id: rustigram_types::user::ChatId,
        _text: String,
    ) -> anyhow::Result<()> {
        self.calls.lock().unwrap().push(TgCall {});

        Ok(())
    }
}

pub struct TestApp {
    pub router: Router,
    pub tg_calls: Arc<Mutex<Vec<TgCall>>>,
    pub tg_update_job_wp: WorkerPool<TgUpdateJob>,
}

impl TestApp {
    pub async fn new(db: sqlx::MySqlPool) -> anyhow::Result<Self> {
        let user_store = MySqlUserStore::new(db.clone());
        let tg_bot_client = TestBotClient::new();
        let tg_calls = tg_bot_client.calls.clone();

        let (tg_update_service, tg_update_job_wp) =
            TgUpdateService::new(tg_bot_client.clone(), user_store.clone(), 1, 10);

        let app_state = AppState::new(
            TG_TOKEN,
            TG_WEBHOOK_SECRET_TOKEN,
            Sessions::new(SESSION_KEY),
            tg_bot_client,
            user_store,
            tg_update_service,
        );

        let router = app_router(app_state);

        Ok(Self {
            router,
            tg_calls,
            tg_update_job_wp,
        })
    }
}
