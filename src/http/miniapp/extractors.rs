use axum::extract::{FromRef, FromRequestParts};
use rustigram_miniapp::{HmacValidateOpts, WebAppInitData, validate_hmac};
use tower_cookies::Cookies;

use crate::{
    app_state::{SessionState, TgBotToken},
    http::miniapp::MiniAppErr,
    security::session::{SESSION_TTL_SECONDS, SessionIdentity},
};

pub const SESSION_COOKIE_NAME: &str = "__Host-yalom-session";

pub struct ValidatedInitData(pub WebAppInitData);

impl<S> FromRequestParts<S> for ValidatedInitData
where
    S: Send + Sync,
    TgBotToken: FromRef<S>,
{
    type Rejection = MiniAppErr;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let TgBotToken(tg_bot_token) = TgBotToken::from_ref(state);

        let raw_init_data = parts
            .headers
            .get("X-Tma-Init-Data")
            .ok_or(MiniAppErr::InvalidInitData)?
            .to_str()
            .map_err(|_| MiniAppErr::InvalidInitData)?;

        let data = validate_hmac(
            raw_init_data,
            &tg_bot_token,
            HmacValidateOpts {
                max_age_secs: Some(SESSION_TTL_SECONDS as u64),
            },
        )
        .map_err(|_| MiniAppErr::InvalidInitData)?;

        Ok(ValidatedInitData(data))
    }
}

impl<S> FromRequestParts<S> for SessionIdentity
where
    S: Send + Sync,
    SessionState: FromRef<S>,
{
    type Rejection = MiniAppErr;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let cookies = Cookies::from_request_parts(parts, state)
            .await
            .map_err(|_| MiniAppErr::MissingSession)?;

        let auth_token = cookies
            .get(SESSION_COOKIE_NAME)
            .ok_or(MiniAppErr::MissingSession)?;

        let SessionState(sessions) = SessionState::from_ref(state);

        let identity = sessions
            .validate(auth_token.value())
            .map_err(MiniAppErr::InvalidSession)?;

        Ok(identity)
    }
}
