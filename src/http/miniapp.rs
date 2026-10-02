use axum::{
    Extension, Router,
    extract::{Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use rustigram_miniapp::{HmacValidateOpts, WebAppInitData, validate_hmac};
use tower_cookies::{
    Cookie, Cookies,
    cookie::{SameSite, time::Duration},
};

use crate::{
    app_state::session_state::MiniAppState,
    security::session::{SESSION_TTL_SECONDS, SessionError, SessionIdentity},
};

pub const SESSION_COOKIE_NAME: &str = "__Host-yalom-session";

#[derive(Debug, thiserror::Error)]
enum MiniAppErr {
    #[error("failed to issue session token")]
    SessionIssue(#[source] SessionError),

    #[error("invalid session token")]
    InvalidSession(#[source] SessionError),

    #[error("missing session token")]
    MissingSession,

    #[error("missing Telegram user data")]
    MissingUser,

    #[error("invalid initData")]
    InvalidInitData,
}

impl IntoResponse for MiniAppErr {
    fn into_response(self) -> Response {
        match self {
            MiniAppErr::SessionIssue(err) => {
                tracing::error!(error = %err);
                StatusCode::INTERNAL_SERVER_ERROR
            }
            MiniAppErr::InvalidSession(_) | MiniAppErr::MissingSession => StatusCode::UNAUTHORIZED,
            MiniAppErr::MissingUser | MiniAppErr::InvalidInitData => StatusCode::BAD_REQUEST,
        }
        .into_response()
    }
}

pub fn router(tg_bot_token: &str, miniapp_state: MiniAppState) -> Router {
    let api_router = Router::new()
        .route(
            "/session",
            post(issue_session)
                .route_layer(middleware::from_fn_with_state(
                    tg_bot_token.to_owned(),
                    validate_init_data,
                ))
                // .route_layer(BotTokenLayer(BotToken(tg_bot_token.to_owned())))
                .with_state(miniapp_state.clone()),
        )
        .route(
            "/menu",
            get(get_menu).route_layer(middleware::from_fn_with_state(
                miniapp_state.clone(),
                validate_session_token,
            )),
        );
    Router::new().nest("/api/miniapp", api_router)
}

async fn issue_session(
    State(state): State<MiniAppState>,
    Extension(init_data): Extension<WebAppInitData>,
    cookies: Cookies,
) -> Result<StatusCode, MiniAppErr> {
    let Some(user) = init_data.user else {
        return Err(MiniAppErr::MissingUser);
    };
    let token = state
        .session_tokens
        .issue(user.id)
        .map_err(MiniAppErr::SessionIssue)?;

    let cookie = Cookie::build((SESSION_COOKIE_NAME, token))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(Duration::seconds(SESSION_TTL_SECONDS))
        .build();

    cookies.add(cookie);

    Ok(StatusCode::NO_CONTENT)
}

async fn validate_session_token(
    State(state): State<MiniAppState>,
    cookies: Cookies,
    mut request: Request,
    next: Next,
) -> Result<Response, MiniAppErr> {
    let auth_token = cookies
        .get(SESSION_COOKIE_NAME)
        .ok_or(MiniAppErr::MissingSession)?;

    let identity = state
        .session_tokens
        .validate(auth_token.value())
        .map_err(MiniAppErr::InvalidSession)?;

    request.extensions_mut().insert(identity);

    Ok(next.run(request).await)
}

// TODO: Replace with Rustigram middleware once https://github.com/meh7an/rustigram/issues/28 is resolved.
async fn validate_init_data(
    State(bot_token): State<String>,
    mut request: Request,
    next: Next,
) -> Result<Response, MiniAppErr> {
    let raw_init_data = request
        .headers()
        .get("X-Tma-Init-Data")
        .ok_or(MiniAppErr::InvalidInitData)?
        .to_str()
        .map_err(|_| MiniAppErr::InvalidInitData)?;

    let data = validate_hmac(
        raw_init_data,
        &bot_token,
        HmacValidateOpts {
            max_age_secs: Some(SESSION_TTL_SECONDS as u64),
        },
    )
    .map_err(|_| MiniAppErr::InvalidInitData)?;

    request.extensions_mut().insert(data);

    Ok(next.run(request).await)
}

async fn get_menu(Extension(_identity): Extension<SessionIdentity>) -> Result<(), StatusCode> {
    todo!();
}
