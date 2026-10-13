pub mod helper;
pub mod tma;

use std::sync::{Arc, Mutex};

use axum::Router;
use yalom_bot::{
    app_state::AppState,
    http::app_router,
    nudge::{
        NudgeService,
        open_ai_adapter::{AdapterError, LlmAdapter, LlmRequest, LlmResponse},
        rate_limit::ManualNudgeMySqlLimiter,
        store::NudgeMySqlStore,
    },
    security::session::Sessions,
    telegram::{
        TelegramClient,
        sender::{TgSenderJob, TgSenderService},
    },
    user::store::MySqlUserStore,
    worker_pool::WorkerPool,
};

pub const TG_WEBHOOK_SECRET_TOKEN: &str = "tg_webhook_secret_token";
pub const TG_TOKEN: &str = "123456789:ABCdefGhIJKlmNoPQRsTUVwxYZ";
pub const SESSION_KEY: &str = "miniapp_session_key";

pub struct TgCall {
    pub chat_id: rustigram_types::user::ChatId,
    pub text: String,
}

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
        chat_id: rustigram_types::user::ChatId,
        text: String,
    ) -> anyhow::Result<()> {
        self.calls.lock().unwrap().push(TgCall { chat_id, text });
        Ok(())
    }
}

pub const FAKE_NUDGE_TEXT: &str = "Тестовое сообщение Yalom";

#[derive(Default)]
pub struct OpenAiFakeAdapter {
    pub calls: Arc<Mutex<Vec<LlmRequest>>>,
}

impl LlmAdapter for OpenAiFakeAdapter {
    async fn execute(&self, request: LlmRequest) -> Result<LlmResponse, AdapterError> {
        self.calls.lock().unwrap().push(request);

        Ok(LlmResponse {
            text: FAKE_NUDGE_TEXT.to_owned(),
        })
    }
}

pub struct TestApp {
    pub router: Router,
    pub tg_calls: Arc<Mutex<Vec<TgCall>>>,
    pub tg_update_job_wp: WorkerPool<TgSenderJob>,
    pub llm_calls: Arc<Mutex<Vec<LlmRequest>>>,
}

impl TestApp {
    pub async fn new(db: sqlx::MySqlPool) -> anyhow::Result<Self> {
        let user_store = MySqlUserStore::new(db.clone());
        let tg_bot_client = TestBotClient::new();
        let tg_calls = tg_bot_client.calls.clone();

        let (tg_update_service, tg_update_job_wp) =
            TgSenderService::new(tg_bot_client.clone(), user_store.clone(), 1, 10);

        let llm_adapter = OpenAiFakeAdapter::default();
        let llm_calls = llm_adapter.calls.clone();

        let nudge_service = NudgeService::new(llm_adapter, NudgeMySqlStore::new(db.clone()));
        let nudge_limiter = ManualNudgeMySqlLimiter::new(db);

        let app_state = AppState::new(
            TG_TOKEN,
            TG_WEBHOOK_SECRET_TOKEN,
            Sessions::new(SESSION_KEY),
            tg_bot_client,
            user_store,
            tg_update_service,
            nudge_service,
            nudge_limiter,
        );

        let router = app_router(app_state);

        Ok(Self {
            router,
            tg_calls,
            llm_calls,
            tg_update_job_wp,
        })
    }
}
