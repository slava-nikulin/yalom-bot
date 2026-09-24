use axum::{
    Extension, Router,
    extract::{Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, get_service, post},
};
use rustigram_miniapp::{BotToken, BotTokenLayer, TmaInitData};
use tower_cookies::{
    Cookie, Cookies,
    cookie::{SameSite, time::Duration},
};
use tower_http::services::{ServeDir, ServeFile};

use crate::{
    app_state::session_state::MiniAppState,
    security::session::{SESSION_TTL_SECONDS, SessionError, SessionIdentity},
};

pub const SESSION_COOKIE_NAME: &str = "__Host-yalom-session";

#[derive(Debug, thiserror::Error)]
enum MiniAppError {
    #[error("failed to issue session token")]
    SessionIssue(#[source] SessionError),

    #[error("invalid session token")]
    InvalidSession(#[source] SessionError),

    #[error("missing session token")]
    MissingSession,

    #[error("missing Telegram user data")]
    MissingUser,
}

impl IntoResponse for MiniAppError {
    fn into_response(self) -> Response {
        match self {
            MiniAppError::SessionIssue(err) => {
                tracing::error!(error = %err);
                StatusCode::INTERNAL_SERVER_ERROR
            }
            MiniAppError::InvalidSession(_) | MiniAppError::MissingSession => {
                StatusCode::UNAUTHORIZED
            }
            MiniAppError::MissingUser => StatusCode::BAD_REQUEST,
        }
        .into_response()
    }
}

pub fn router(tg_bot_token: &str, miniapp_state: MiniAppState) -> Router {
    //TODO: configure initData max age when rustigram exposes it in TmaInitData https://github.com/meh7an/rustigram/issues/28
    let static_files = Router::new().fallback_service(ServeDir::new("web/miniapp/dist/client"));

    Router::new()
        .route_service(
            "/miniapp",
            get_service(ServeFile::new("web/miniapp/dist/client/index.html")),
        )
        .route_service(
            "/miniapp/",
            get_service(ServeFile::new("web/miniapp/dist/client/index.html")),
        )
        .route(
            "/miniapp/session",
            post(issue_session)
                .route_layer(BotTokenLayer(BotToken(tg_bot_token.to_owned())))
                .with_state(miniapp_state.clone()),
        )
        .route(
            "/miniapp/menu",
            get(get_menu).route_layer(middleware::from_fn_with_state(
                miniapp_state.clone(),
                validate_session_token,
            )),
        )
        .nest("/miniapp", static_files)
}

async fn issue_session(
    State(state): State<MiniAppState>,
    TmaInitData(init_data): TmaInitData,
    cookies: Cookies,
) -> Result<StatusCode, MiniAppError> {
    let Some(user) = init_data.user else {
        return Err(MiniAppError::MissingUser);
    };
    let token = state
        .session_tokens
        .issue(user.id)
        .map_err(MiniAppError::SessionIssue)?;

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
) -> Result<Response, MiniAppError> {
    let auth_token = cookies
        .get(SESSION_COOKIE_NAME)
        .ok_or(MiniAppError::MissingSession)?;

    let identity = state
        .session_tokens
        .validate(auth_token.value())
        .map_err(MiniAppError::InvalidSession)?;

    request.extensions_mut().insert(identity);

    Ok(next.run(request).await)
}

async fn get_menu(Extension(_identity): Extension<SessionIdentity>) -> Result<(), StatusCode> {
    todo!();
}
