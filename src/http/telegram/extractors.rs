use axum::{
    extract::{FromRef, FromRequestParts},
    http::StatusCode,
};

use crate::app_state::TgWebhookSecret;
use constant_time_eq::constant_time_eq;

pub struct ValidWebhookSecret;

impl<S> FromRequestParts<S> for ValidWebhookSecret
where
    S: Send + Sync,
    TgWebhookSecret: FromRef<S>,
{
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        const HEADER: &str = "x-telegram-bot-api-secret-token";

        let TgWebhookSecret(tg_webhook_token) = TgWebhookSecret::from_ref(state);

        let valid = parts
            .headers
            .get(HEADER)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| constant_time_eq(value.as_bytes(), tg_webhook_token.as_bytes()));

        if !valid {
            return Err(StatusCode::UNAUTHORIZED);
        }

        Ok(ValidWebhookSecret)
    }
}
