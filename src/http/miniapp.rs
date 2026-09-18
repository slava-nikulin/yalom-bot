use axum::{
    Extension, Router,
    extract::{Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use rustigram_miniapp::{BotToken, BotTokenLayer, TmaInitData};
use tower_cookies::{
    Cookie, Cookies,
    cookie::{SameSite, time::Duration},
};

use crate::{
    app_state::session_state::MiniAppState,
    security::session::{SessionIdentity, SessionIssueError, SessionValidationError},
};

const SESSION_COOKIE_NAME: &str = "__Host-yalom-session";
pub const SESSION_TTL_SECONDS: i64 = 30 * 60;

#[derive(Debug, thiserror::Error)]
enum MiniAppError {
    #[error("unauthorized")]
    Unauthorized,

    #[error("internal server error")]
    Internal(#[from] SessionIssueError),
}

impl From<SessionValidationError> for MiniAppError {
    fn from(_: SessionValidationError) -> Self {
        Self::Unauthorized
    }
}

impl IntoResponse for MiniAppError {
    fn into_response(self) -> Response {
        match self {
            MiniAppError::Unauthorized => StatusCode::UNAUTHORIZED,
            MiniAppError::Internal(err) => {
                tracing::error!(error = %err);
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
        .into_response()
    }
}

pub fn router(session_key: &str, miniapp_state: MiniAppState) -> Router {
    Router::new()
        .route(
            "/miniapp/session",
            post(issue_session)
                .route_layer(BotTokenLayer(BotToken(session_key.to_owned())))
                .with_state(miniapp_state.clone()),
        )
        .route(
            "/miniapp/menu",
            get(get_menu).route_layer(middleware::from_fn_with_state(
                miniapp_state.clone(),
                validate_session_token,
            )),
        )
}

async fn issue_session(
    State(state): State<MiniAppState>,
    TmaInitData(init_data): TmaInitData,
    cookies: Cookies,
) -> Result<StatusCode, MiniAppError> {
    let Some(user) = init_data.user else {
        return Err(MiniAppError::Unauthorized);
    };
    let token = state.session_tokens.issue(user.id, SESSION_TTL_SECONDS)?;

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
        .ok_or(MiniAppError::Unauthorized)?;

    let identity = state.session_tokens.validate(auth_token.value())?;

    request.extensions_mut().insert(identity);

    Ok(next.run(request).await)
}

async fn get_menu(Extension(_identity): Extension<SessionIdentity>) -> Result<(), StatusCode> {
    todo!();
}
