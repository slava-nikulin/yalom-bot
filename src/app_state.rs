use std::sync::Arc;

use axum::extract::FromRef;

use crate::{
    nudge::{
        NudgeService, open_ai_adapter::LlmAdapter, rate_limit::ManualNudgeMySqlLimiter,
        store::NudgeStore,
    },
    security::session::Sessions,
    telegram::sender::TgSenderService,
    user::store::UserStore,
};

#[derive(Clone)]
pub struct UserStoreState<Us>(pub Us);

#[derive(Clone)]
pub struct TgBotToken(pub Arc<str>);

#[derive(Clone)]
pub struct TgWebhookSecret(pub Arc<str>);

pub struct AppState<Us, A, Ns>
where
    A: LlmAdapter,
    Ns: NudgeStore,
{
    pub sessions: Arc<Sessions>,
    pub user_store: UserStoreState<Us>,
    pub tg_bot_token: TgBotToken,
    pub tg_webhook_secret: TgWebhookSecret,
    pub tg_update_service: TgSenderService,
    pub nudge_service: Arc<NudgeService<A, Ns>>,
    pub nudge_limiter: ManualNudgeMySqlLimiter,
}

impl<Us, A, Ns> Clone for AppState<Us, A, Ns>
where
    Us: UserStore,
    A: LlmAdapter,
    Ns: NudgeStore,
{
    fn clone(&self) -> Self {
        Self {
            sessions: self.sessions.clone(),
            user_store: self.user_store.clone(),
            tg_bot_token: self.tg_bot_token.clone(),
            tg_webhook_secret: self.tg_webhook_secret.clone(),
            tg_update_service: self.tg_update_service.clone(),
            nudge_service: Arc::clone(&self.nudge_service),
            nudge_limiter: self.nudge_limiter.clone(),
        }
    }
}

impl<Us, A, Ns> FromRef<AppState<Us, A, Ns>> for UserStoreState<Us>
where
    Us: UserStore,
    A: LlmAdapter,
    Ns: NudgeStore,
{
    fn from_ref(input: &AppState<Us, A, Ns>) -> Self {
        input.user_store.clone()
    }
}

impl<Us, A, Ns> FromRef<AppState<Us, A, Ns>> for Arc<Sessions>
where
    Us: UserStore,
    A: LlmAdapter,
    Ns: NudgeStore,
{
    fn from_ref(input: &AppState<Us, A, Ns>) -> Self {
        input.sessions.clone()
    }
}

impl<Us, A, Ns> FromRef<AppState<Us, A, Ns>> for TgBotToken
where
    Us: UserStore,
    A: LlmAdapter,
    Ns: NudgeStore,
{
    fn from_ref(input: &AppState<Us, A, Ns>) -> Self {
        input.tg_bot_token.clone()
    }
}

impl<Us, A, Ns> FromRef<AppState<Us, A, Ns>> for TgWebhookSecret
where
    Us: UserStore,
    A: LlmAdapter,
    Ns: NudgeStore,
{
    fn from_ref(input: &AppState<Us, A, Ns>) -> Self {
        input.tg_webhook_secret.clone()
    }
}

impl<Us, A, Ns> FromRef<AppState<Us, A, Ns>> for TgSenderService
where
    Us: UserStore,
    A: LlmAdapter,
    Ns: NudgeStore,
{
    fn from_ref(input: &AppState<Us, A, Ns>) -> Self {
        input.tg_update_service.clone()
    }
}

impl<Us, A, Ns> FromRef<AppState<Us, A, Ns>> for Arc<NudgeService<A, Ns>>
where
    Us: UserStore,
    A: LlmAdapter,
    Ns: NudgeStore,
{
    fn from_ref(input: &AppState<Us, A, Ns>) -> Self {
        Arc::clone(&input.nudge_service)
    }
}

impl<Us, A, Ns> FromRef<AppState<Us, A, Ns>> for ManualNudgeMySqlLimiter
where
    Us: UserStore,
    A: LlmAdapter,
    Ns: NudgeStore,
{
    fn from_ref(input: &AppState<Us, A, Ns>) -> Self {
        input.nudge_limiter.clone()
    }
}
