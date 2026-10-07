pub mod extractors;

use axum::{
    Json, Router,
    extract::{FromRef, FromRequestParts, State},
    http::StatusCode,
    routing::post,
};

use rustigram_types::{Update, UpdateKind};

use crate::{
    http::telegram::extractors::ValidWebhookSecret,
    telegram::update_service::{TgUpdateJob, TgUpdateService},
};

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    TgUpdateService: FromRef<S>,
    ValidWebhookSecret: FromRequestParts<S>,
{
    Router::<S>::new().route("/tg/webhook", post(tg_webhook))
}

async fn tg_webhook(
    _: ValidWebhookSecret,
    State(state): State<TgUpdateService>,
    Json(update): Json<Update>,
) -> StatusCode {
    let UpdateKind::Message(message) = update.kind else {
        return StatusCode::OK;
    };

    tracing::info!(update_id = update.update_id, chat_id = %message.chat.id, "telegram update received");

    let tg_job = if message
        .command()
        .is_some_and(|c| c.eq_ignore_ascii_case("start"))
    {
        let Some(ref tg_user) = message.from else {
            return StatusCode::OK;
        };

        TgUpdateJob::Start {
            tg_user_id: tg_user.id,
            chat_id: message.chat.id,
        }
    } else {
        let Some(text) = message.text else {
            return StatusCode::OK;
        };

        TgUpdateJob::Message {
            chat_id: message.chat.id,
            text,
        }
    };

    // TODO: durable inbox
    if let Err(err) = state.enqueue(tg_job).await {
        tracing::error!(
            error = %err,
            chat_id = %message.chat.id,
            "failed to enqueue telegram update"
        );

        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::OK
    }
}
