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
    security::session::{SessionIdentity, SessionIssueError},
};

const SESSION_COOKIE_NAME: &str = "__Host-yalom-session";
const SESSION_TTL: Duration = Duration::minutes(30);

#[derive(Debug, thiserror::Error)]
enum MiniAppError {
    #[error("unauthorized")]
    Unauthorized,

    #[error("internal server error")]
    Internal(#[from] SessionIssueError),
}

impl IntoResponse for MiniAppError {
    fn into_response(self) -> Response {
        (
            match self {
                MiniAppError::Unauthorized => StatusCode::UNAUTHORIZED,
                MiniAppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            },
            "",
        )
            .into_response()
    }
}

pub fn router(tg_token: &str, miniapp_state: MiniAppState) -> Router {
    Router::new()
        .route(
            "/miniapp/session",
            post(issue_session)
                .route_layer(BotTokenLayer(BotToken(tg_token.to_owned())))
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
    let token = state.session_tokens.issue(user.id)?;

    let cookie = Cookie::build((SESSION_COOKIE_NAME, token))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(SESSION_TTL)
        .build();

    cookies.add(cookie);

    Ok(StatusCode::OK)
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

    let identity = state
        .session_tokens
        .validate(auth_token.value())
        .map_err(|_| MiniAppError::Unauthorized)?;

    request.extensions_mut().insert(identity);

    Ok(next.run(request).await)
}

async fn get_menu(Extension(_identity): Extension<SessionIdentity>) -> Result<(), StatusCode> {
    // get user from identity
    Ok(())
}
