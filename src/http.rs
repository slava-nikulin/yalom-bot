use axum::{
    Json, Router,
    extract::{Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
};
use constant_time_eq::constant_time_eq;
use rustigram_miniapp::BotToken;
use rustigram_miniapp::{BotTokenLayer, TmaInitData};
use rustigram_types::{Update, UpdateKind};
use std::sync::Arc;
use tower_cookies::{Cookie, CookieManagerLayer, Cookies, cookie::SameSite};

use crate::{app_state::AppState, telegram::TelegramClient};

pub fn app_router<Tg: TelegramClient>(
    tg_webhook_secret: String,
    tg_token: &str,
    app_state: AppState<Tg>,
) -> Router {
    Router::new()
        .route("/health", get(health))
        .route(
            "/tg/webhook",
            post(tg_webhook::<Tg>)
                .route_layer(middleware::from_fn_with_state(
                    Arc::<str>::from(tg_webhook_secret),
                    verify_tg_webhook_secret,
                ))
                .with_state(app_state.clone()),
        )
        .route(
            "/miniapp/session",
            post(issue_session)
                .route_layer(BotTokenLayer(BotToken(tg_token.to_owned())))
                .with_state(app_state.clone()),
        )
        .route(
            "/miniapp/menu",
            get(get_menu).route_layer(middleware::from_fn_with_state(
                app_state.clone(),
                validate_session_token,
            )),
        )
        .layer(CookieManagerLayer::new())
}

async fn issue_session<Tg: TelegramClient>(
    State(state): State<AppState<Tg>>,
    TmaInitData(init_data): TmaInitData,
    cookies: Cookies,
) -> Result<StatusCode, StatusCode> {
    let token = state
        .session_tokens
        .build_paseto_token(init_data.user.ok_or(StatusCode::UNAUTHORIZED)?.id)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let cookie = Cookie::build(("auth_token", token))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Lax)
        .path("/")
        .build();

    cookies.add(cookie);

    Ok(StatusCode::OK)
}

async fn validate_session_token<Tg: TelegramClient>(
    State(state): State<AppState<Tg>>,
    cookies: Cookies,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_token = cookies.get("auth_token").ok_or(StatusCode::UNAUTHORIZED)?;

    state
        .session_tokens
        .validate_paseto_token(auth_token.value())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(next.run(request).await)
}

async fn get_menu() -> Result<(), StatusCode> {
    // get user.
    Ok(())
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

async fn health() -> StatusCode {
    StatusCode::OK
}
