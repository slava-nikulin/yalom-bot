pub mod health;
pub mod miniapp;
pub mod telegram;

use axum::{Router, routing::get};

use tower_cookies::CookieManagerLayer;

use crate::{
    app_state::{AppState, session_state::MiniAppState},
    telegram::TelegramClient,
};

pub fn app_router<Tg: TelegramClient>(
    tg_bot_token: &str,
    tg_webhook_secret: String,
    app_state: AppState<Tg>,
    miniapp_state: MiniAppState,
) -> Router {
    Router::new()
        .route("/health", get(health::health))
        .merge(miniapp::router(tg_bot_token, miniapp_state))
        .merge(telegram::router::<Tg>(tg_webhook_secret, app_state))
        .layer(CookieManagerLayer::new())
}
