use rustigram_api::BotClient;
use std::sync::Arc;

pub struct AppState {
    pub telegram: BotClient,
}

pub type SharedState = Arc<AppState>;

impl AppState {
    pub fn new(telegram: BotClient) -> SharedState {
        Arc::new(AppState { telegram })
    }
}
