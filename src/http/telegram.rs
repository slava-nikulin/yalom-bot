use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::Response,
    routing::post,
};

use constant_time_eq::constant_time_eq;
use rustigram_types::{Update, UpdateKind};

use crate::{app_state::AppState, telegram::TelegramClient};

pub fn router<Tg: TelegramClient>(tg_webhook_secret: String, app_state: AppState<Tg>) -> Router {
    Router::new().route(
        "/tg/webhook",
        post(tg_webhook::<Tg>)
            .route_layer(middleware::from_fn_with_state(
                Arc::<str>::from(tg_webhook_secret),
                verify_tg_webhook_secret,
            ))
            .with_state(app_state.clone()),
    )
}

async fn verify_tg_webhook_secret(
    State(tg_webhook_token): State<Arc<str>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    const HEADER: &str = "x-telegram-bot-api-secret-token";

    let valid = request
        .headers()
        .get(HEADER)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| constant_time_eq(value.as_bytes(), tg_webhook_token.as_bytes()));

    if !valid {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(request).await)
}

async fn tg_webhook<Tg: TelegramClient>(
    State(state): State<AppState<Tg>>,
    Json(update): Json<Update>,
) -> StatusCode {
    tracing::info!(update_id = update.update_id, "telegram update received");

    let UpdateKind::Message(message) = update.kind else {
        return StatusCode::OK;
    };

    let Some(text) = message.text else {
        return StatusCode::OK;
    };

    let chat_id = message.chat.id;

    if let Err(err) = state
        .telegram
        .send_message(
            rustigram_types::ChatId::Id(chat_id),
            format!("you said {}", text),
        )
        .await
    {
        tracing::error!(
            error = %err,
            chat_id,
            "failed to send telegram message"
        );
    }

    StatusCode::OK
}
