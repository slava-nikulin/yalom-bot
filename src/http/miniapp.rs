pub mod dto;
pub mod extractors;

use axum::{
    Json, Router,
    extract::{FromRef, FromRequestParts, State},
    http::{HeaderValue, StatusCode, header::RETRY_AFTER},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use chrono::Utc;
use chrono_tz::Tz::{self};
use std::{sync::Arc, time::Duration as StdDuration};
use tower_cookies::{
    Cookie, Cookies,
    cookie::{SameSite, time::Duration},
};

use crate::{
    app_state::UserStoreState,
    http::miniapp::{
        dto::{ErrorBody, GenerateNudgeRequest, MenuState, NudgeResponse, UpdateMenuRequest},
        extractors::{SESSION_COOKIE_NAME, ValidatedInitData},
    },
    nudge::{
        DefaultRng, NudgeService,
        model::NudgeTime,
        open_ai_adapter::LlmAdapter,
        rate_limit::{ManualNudgeMySqlLimiter, RateLimitDecision, RateLimitReason},
        store::NudgeStore,
    },
    security::session::{SESSION_TTL_SECONDS, SessionError, SessionIdentity, Sessions},
    telegram::sender::{TgSenderJob, TgSenderService},
    user::store::{UserStore, UserStoreError},
};

#[derive(Debug, thiserror::Error)]
pub enum MiniAppErr {
    #[error("internal_error")]
    SessionIssue(#[source] SessionError),

    #[error("unauthorized")]
    InvalidSession(#[source] SessionError),

    #[error("unauthorized")]
    MissingSession,

    #[error("missing_user_data")]
    MissingUserData,

    #[error("invalid_init_data")]
    InvalidInitData,

    #[error("user_not_found")]
    UserNotFound,

    #[error("invalid_timezone")]
    InvalidTimezone,

    #[error("inappropriate_time")]
    InappropriateTime,

    #[error("rate_limited")]
    Cooldown(StdDuration),

    #[error("daily_limit")]
    DailyLimit(StdDuration),

    #[error("internal_error")]
    Internal,

    #[error("service_unavailable")]
    Unavailable,
}

impl IntoResponse for MiniAppErr {
    fn into_response(self) -> Response {
        if let Self::SessionIssue(err) = &self {
            tracing::error!(error = %err, "failed to issue session");
        }

        let status = match &self {
            Self::SessionIssue(_) | Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,

            Self::InvalidSession(_) | Self::MissingSession => StatusCode::UNAUTHORIZED,

            Self::MissingUserData | Self::InvalidInitData | Self::InvalidTimezone => {
                StatusCode::BAD_REQUEST
            }

            Self::UserNotFound => StatusCode::NOT_FOUND,

            Self::InappropriateTime => StatusCode::UNPROCESSABLE_ENTITY,

            Self::Cooldown(_) | Self::DailyLimit(_) => StatusCode::TOO_MANY_REQUESTS,

            Self::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        };

        let retry_after_seconds = match &self {
            Self::Cooldown(duration) | Self::DailyLimit(duration) => {
                Some(duration.as_secs() + u64::from(duration.subsec_nanos() > 0))
            }
            _ => None,
        };

        let mut response = (
            status,
            Json(ErrorBody {
                code: self.to_string(),
                retry_after_seconds,
            }),
        )
            .into_response();

        if let Some(seconds) = retry_after_seconds {
            response.headers_mut().insert(
                RETRY_AFTER,
                HeaderValue::from_str(&seconds.to_string()).expect("valid retry-after"),
            );
        }

        response
    }
}

pub fn router<S, Us, A, Ns>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    Us: UserStore,
    Ns: NudgeStore,
    A: LlmAdapter,
    UserStoreState<Us>: FromRef<S>,
    Arc<Sessions>: FromRef<S>,
    Arc<NudgeService<A, Ns>>: FromRef<S>,
    ManualNudgeMySqlLimiter: FromRef<S>,
    TgSenderService: FromRef<S>,
    ValidatedInitData: FromRequestParts<S>,
{
    let api_router = Router::new()
        .route("/session", post(issue_session))
        .route("/menu", get(get_menu).patch(update_bot_settings))
        .route("/nudge", post(generate_nudge_manual));
    Router::new().nest("/api/miniapp", api_router)
}

async fn issue_session(
    State(state): State<Arc<Sessions>>,
    ValidatedInitData(init_data): ValidatedInitData,
    cookies: Cookies,
) -> Result<StatusCode, MiniAppErr> {
    let Some(tg_user) = init_data.user else {
        return Err(MiniAppErr::MissingUserData);
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
    State(UserStoreState(user_store)): State<UserStoreState<Us>>,
    State(nudge_service): State<Arc<NudgeService<A, Ns>>>,
    State(limiter): State<ManualNudgeMySqlLimiter>,
    State(tg_service): State<TgSenderService>,
    Json(request): Json<GenerateNudgeRequest>,
) -> Result<Json<NudgeResponse>, MiniAppErr>
where
    Us: UserStore,
    A: LlmAdapter,
    Ns: NudgeStore,
{
    let user = user_store
        .get(identity.tg_user_id)
        .await
        .map_err(|err| match err {
            UserStoreError::NotFound => MiniAppErr::UserNotFound,
            other => {
                tracing::error!(error = ?other, "failed to load user");
                MiniAppErr::Internal
            }
        })?;

    let tz: Tz = request
        .timezone
        .parse()
        .map_err(|_| MiniAppErr::InvalidTimezone)?;

    let nudge_time = NudgeTime::try_from(Utc::now().with_timezone(&tz))
        .map_err(|_| MiniAppErr::InappropriateTime)?;

    match limiter.acquire(identity.tg_user_id).await {
        Ok(RateLimitDecision::Allowed) => {}

        Ok(RateLimitDecision::Rejected {
            reason,
            retry_after,
        }) => {
            return Err(match reason {
                RateLimitReason::Cooldown => MiniAppErr::Cooldown(retry_after),

                RateLimitReason::DailyQuota => MiniAppErr::DailyLimit(retry_after),
            });
        }

        Err(err) => {
            tracing::error!(
                error = %err,
                tg_user_id = identity.tg_user_id,
                "rate limiter failed"
            );

            return Err(MiniAppErr::Internal);
        }
    }

    let nudge = nudge_service
        .generate(DefaultRng, nudge_time)
        .await
        .map_err(|err| {
            tracing::error!(
                error = ?err,
                tg_user_id = identity.tg_user_id,
                "failed to generate nudge"
            );

            MiniAppErr::Internal
        })?;

    tg_service
        .enqueue(TgSenderJob::Nudge {
            chat_id: user.chat_id,
            text: nudge.text.clone(),
        })
        .await
        .map_err(|err| {
            tracing::error!(
                error = %err,
                tg_user_id = identity.tg_user_id,
                "failed to enqueue nudge"
            );

            MiniAppErr::Unavailable
        })?;

    Ok(Json(NudgeResponse { text: nudge.text }))
}
