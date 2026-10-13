pub mod health;
pub mod miniapp;
pub mod telegram;

use axum::{Router, routing::get};

use tower_cookies::CookieManagerLayer;

use crate::{
    app_state::AppState,
    nudge::{open_ai_adapter::LlmAdapter, store::NudgeStore},
    user::store::UserStore,
};

pub fn app_router<Us, A, Ns>(app_state: AppState<Us, A, Ns>) -> Router
where
    Us: UserStore,
    A: LlmAdapter,
    Ns: NudgeStore,
{
    Router::new()
        .route("/health", get(health::health))
        .merge(miniapp::router::<AppState<Us, A, Ns>, Us, A, Ns>())
        .merge(telegram::router::<AppState<Us, A, Ns>>())
        .layer(CookieManagerLayer::new())
        .with_state(app_state)
}
