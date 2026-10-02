pub mod helper;
pub mod tma;

use std::sync::{Arc, Mutex};

use axum::Router;
use yalom_bot::{
    app_state::{AppState, session_state::MiniAppState},
    http::app_router,
    security::session::SessionTokens,
    telegram::TelegramClient,
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
}

impl TestApp {
    pub fn new() -> Self {
        let tg_bot_client = TestBotClient::new();
        let tg_calls = tg_bot_client.calls.clone();

        let app_state = AppState::new(tg_bot_client);

        let miniapp_state = MiniAppState::new(SessionTokens::new(SESSION_KEY));

        let router = app_router(
            TG_TOKEN,
            TG_WEBHOOK_SECRET_TOKEN.into(),
            app_state,
            miniapp_state,
        );

        Self { router, tg_calls }
    }
}
