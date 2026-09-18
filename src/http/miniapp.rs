use axum::{
    Extension, Router,
    extract::{Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
};
use rustigram_miniapp::{BotToken, BotTokenLayer, TmaInitData};
use tower_cookies::{Cookie, Cookies, cookie::SameSite};

use crate::{
    app_state::session_state::MiniAppState, http::miniapp, security::session::SessionIdentity,
};

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
            get(miniapp::get_menu).route_layer(middleware::from_fn_with_state(
                miniapp_state.clone(),
                miniapp::validate_session_token,
            )),
        )
}

async fn issue_session(
    State(state): State<MiniAppState>,
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

async fn validate_session_token(
    State(state): State<MiniAppState>,
    cookies: Cookies,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_token = cookies.get("auth_token").ok_or(StatusCode::UNAUTHORIZED)?;

    let session_identity = state
        .session_tokens
        .validate_paseto_token(auth_token.value())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    request.extensions_mut().insert(session_identity);

    Ok(next.run(request).await)
}

async fn get_menu(Extension(identity): Extension<SessionIdentity>) -> Result<(), StatusCode> {
    // get user from identity
    Ok(())
}
