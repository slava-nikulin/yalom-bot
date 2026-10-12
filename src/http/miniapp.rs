pub mod dto;
pub mod extractors;

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{FromRef, FromRequestParts, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use chrono::Utc;
use chrono_tz::Tz::{self};
use tower_cookies::{
    Cookie, Cookies,
    cookie::{SameSite, time::Duration},
};

use crate::{
    app_state::{SessionState, UserStoreState},
    http::miniapp::{
        dto::{GenerateNudgeRequest, MenuState, NudgeResponse, UpdateMenuRequest},
        extractors::{SESSION_COOKIE_NAME, ValidatedInitData},
    },
    nudge::{DefaultRng, NudgeService, open_ai_adapter::LlmAdapter, store::NudgeStore},
    security::session::{SESSION_TTL_SECONDS, SessionError, SessionIdentity},
    user::store::{UserStore, UserStoreError},
};

#[derive(Debug, thiserror::Error)]
pub enum MiniAppErr {
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

pub fn router<S, Us, A, Ns>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    Us: UserStore,
    Ns: NudgeStore,
    A: LlmAdapter,
    UserStoreState<Us>: FromRef<S>,
    SessionState: FromRef<S>,
    Arc<NudgeService<A, Ns>>: FromRef<S>,
    ValidatedInitData: FromRequestParts<S>,
{
    let api_router = Router::new()
        .route("/session", post(issue_session))
        .route("/menu", get(get_menu).patch(update_bot_settings))
        .route("/nudge", post(generate_nudge_manual));
    Router::new().nest("/api/miniapp", api_router)
}

async fn issue_session(
    State(SessionState(state)): State<SessionState>,
    ValidatedInitData(init_data): ValidatedInitData,
    cookies: Cookies,
) -> Result<StatusCode, MiniAppErr> {
    let Some(tg_user) = init_data.user else {
        return Err(MiniAppErr::MissingUser);
    };
    let token = state.issue(tg_user.id).map_err(MiniAppErr::SessionIssue)?;

    let cookie = Cookie::build((SESSION_COOKIE_NAME, token))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::None)
        .partitioned(true)
        .path("/")
        .max_age(Duration::seconds(SESSION_TTL_SECONDS))
        .build();

    cookies.add(cookie);

    Ok(StatusCode::NO_CONTENT)
}

async fn get_menu<Us: UserStore>(
    identity: SessionIdentity,
    State(UserStoreState(state)): State<UserStoreState<Us>>,
) -> Result<Json<MenuState>, StatusCode> {
    // TODO: получать tz из miniapp(заголовок?) и сохранять в пользователе

    let user = state
        .get(identity.tg_user_id)
        .await
        .map_err(|err| match err {
            UserStoreError::Database(_) => {
                tracing::error!(error = %err, "failed to load user");
                StatusCode::INTERNAL_SERVER_ERROR
            }
            UserStoreError::NotFound => StatusCode::NOT_FOUND,
        })?;

    Ok(Json(MenuState {
        paused: !user.is_active,
    }))
}

async fn update_bot_settings<Us: UserStore>(
    identity: SessionIdentity,
    State(UserStoreState(store)): State<UserStoreState<Us>>,
    Json(request): Json<UpdateMenuRequest>,
) -> Result<Json<MenuState>, StatusCode> {
    store
        .update_bot_settings(identity.tg_user_id, !request.paused)
        .await
        .map_err(|err| match err {
            UserStoreError::Database(_) => {
                tracing::error!(
                    error = %err,
                    tg_user_id = identity.tg_user_id,
                    "failed to update bot settings"
                );

                StatusCode::INTERNAL_SERVER_ERROR
            }
            UserStoreError::NotFound => StatusCode::NOT_FOUND,
        })?;

    Ok(Json(MenuState {
        paused: request.paused,
    }))
}

async fn generate_nudge_manual<Us, A, Ns>(
    identity: SessionIdentity,
    State(UserStoreState(_user_store)): State<UserStoreState<Us>>,
    State(nudge_service): State<Arc<NudgeService<A, Ns>>>,
    Json(request): Json<GenerateNudgeRequest>,
) -> Result<Json<NudgeResponse>, StatusCode>
where
    Us: UserStore,
    A: LlmAdapter,
    Ns: NudgeStore,
{
    let tz: Tz = request
        .timezone
        .parse()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let now = Utc::now().with_timezone(&tz);
    let nudge = nudge_service
        .generate(DefaultRng, now)
        .await
        .map_err(|err| {
            tracing::error!(
                error = %err,
                tg_user_id = identity.tg_user_id,
                "failed to generate the nudge"
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(NudgeResponse { text: nudge.text }))
}
