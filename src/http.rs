pub mod health;
pub mod miniapp;
pub mod telegram;

use axum::{Router, routing::get};

use tower_cookies::CookieManagerLayer;

use crate::{app_state::AppState, telegram::TelegramClient, user::store::UserStore};

pub fn app_router<Tg: TelegramClient, Us: UserStore>(app_state: AppState<Tg, Us>) -> Router {
    Router::new()
        .route("/health", get(health::health))
        .merge(miniapp::router::<AppState<Tg, Us>, Us>())
        .merge(telegram::router::<AppState<Tg, Us>>())
        .layer(CookieManagerLayer::new())
        .with_state(app_state)
}
