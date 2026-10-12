pub mod health;
pub mod miniapp;
pub mod telegram;

use axum::{Router, routing::get};

use tower_cookies::CookieManagerLayer;

use crate::{
    app_state::AppState,
    nudge::{open_ai_adapter::LlmAdapter, store::NudgeStore},
    telegram::TelegramClient,
    user::store::UserStore,
};

pub fn app_router<Tg, Us, A, Ns>(app_state: AppState<Tg, Us, A, Ns>) -> Router
where
    Tg: TelegramClient,
    Us: UserStore,
    A: LlmAdapter,
    Ns: NudgeStore,
{
    Router::new()
        .route("/health", get(health::health))
        .merge(miniapp::router::<AppState<Tg, Us, A, Ns>, Us, A, Ns>())
        .merge(telegram::router::<AppState<Tg, Us, A, Ns>>())
        .layer(CookieManagerLayer::new())
        .with_state(app_state)
}
