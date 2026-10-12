use std::sync::Arc;

use axum::extract::FromRef;

use crate::{
    nudge::{
        NudgeService, open_ai_adapter::LlmAdapter, rate_limit::ManualNudgeMySqlLimiter,
        store::NudgeStore,
    },
    security::session::Sessions,
    telegram::{TelegramClient, sender::TgSenderService},
    user::store::UserStore,
};

#[derive(Clone)]
pub struct TelegramState<Tg>(pub Tg);

#[derive(Clone)]
pub struct UserStoreState<Us>(pub Us);

#[derive(Clone)]
pub struct SessionState(pub Arc<Sessions>);

#[derive(Clone)]
pub struct TgBotToken(pub Arc<str>);

#[derive(Clone)]
pub struct TgWebhookSecret(pub Arc<str>);

pub struct AppState<Tg, Us, A, Ns>
where
    A: LlmAdapter,
    Ns: NudgeStore,
{
    pub telegram: TelegramState<Tg>,
    pub sessions: SessionState,
    pub user_store: UserStoreState<Us>,
    pub tg_bot_token: TgBotToken,
    pub tg_webhook_secret: TgWebhookSecret,
    pub tg_update_service: TgSenderService,
    pub nudge_service: Arc<NudgeService<A, Ns>>,
    pub nudge_limiter: ManualNudgeMySqlLimiter,
}

impl<Tg, Us, A, Ns> AppState<Tg, Us, A, Ns>
where
    Tg: TelegramClient,
    Us: UserStore,
    A: LlmAdapter,
    Ns: NudgeStore,
{
    pub fn new(
        tg_bot_token: &str,
        tg_webhook_secret: &str,
        sessions: Sessions,
        tg_bot: Tg,
        user_store: Us,
        tg_update_service: TgSenderService,
        nudge_service: NudgeService<A, Ns>,
        nudge_limiter: ManualNudgeMySqlLimiter,
    ) -> Self {
        Self {
            telegram: TelegramState::<Tg>(tg_bot),
            user_store: UserStoreState::<Us>(user_store),
            sessions: SessionState(Arc::new(sessions)),
            tg_bot_token: TgBotToken(Arc::from(tg_bot_token)),
            tg_webhook_secret: TgWebhookSecret(Arc::from(tg_webhook_secret)),
            tg_update_service,
            nudge_service: Arc::new(nudge_service),
            nudge_limiter,
        }
    }
}

impl<Tg, Us, A, Ns> Clone for AppState<Tg, Us, A, Ns>
where
    Tg: TelegramClient,
    Us: UserStore,
    A: LlmAdapter,
    Ns: NudgeStore,
{
    fn clone(&self) -> Self {
        Self {
            telegram: self.telegram.clone(),
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

impl<Tg, Us, A, Ns> FromRef<AppState<Tg, Us, A, Ns>> for TelegramState<Tg>
where
    Tg: TelegramClient,
    Us: UserStore,
    A: LlmAdapter,
    Ns: NudgeStore,
{
    fn from_ref(input: &AppState<Tg, Us, A, Ns>) -> Self {
        input.telegram.clone()
    }
}

impl<Tg, Us, A, Ns> FromRef<AppState<Tg, Us, A, Ns>> for UserStoreState<Us>
where
    Tg: TelegramClient,
    Us: UserStore,
    A: LlmAdapter,
    Ns: NudgeStore,
{
    fn from_ref(input: &AppState<Tg, Us, A, Ns>) -> Self {
        input.user_store.clone()
    }
}

impl<Tg, Us, A, Ns> FromRef<AppState<Tg, Us, A, Ns>> for SessionState
where
    Tg: TelegramClient,
    Us: UserStore,
    A: LlmAdapter,
    Ns: NudgeStore,
{
    fn from_ref(input: &AppState<Tg, Us, A, Ns>) -> Self {
        input.sessions.clone()
    }
}

impl<Tg, Us, A, Ns> FromRef<AppState<Tg, Us, A, Ns>> for TgBotToken
where
    Tg: TelegramClient,
    Us: UserStore,
    A: LlmAdapter,
    Ns: NudgeStore,
{
    fn from_ref(input: &AppState<Tg, Us, A, Ns>) -> Self {
        input.tg_bot_token.clone()
    }
}

impl<Tg, Us, A, Ns> FromRef<AppState<Tg, Us, A, Ns>> for TgWebhookSecret
where
    Tg: TelegramClient,
    Us: UserStore,
    A: LlmAdapter,
    Ns: NudgeStore,
{
    fn from_ref(input: &AppState<Tg, Us, A, Ns>) -> Self {
        input.tg_webhook_secret.clone()
    }
}

impl<Tg, Us, A, Ns> FromRef<AppState<Tg, Us, A, Ns>> for TgSenderService
where
    Tg: TelegramClient,
    Us: UserStore,
    A: LlmAdapter,
    Ns: NudgeStore,
{
    fn from_ref(input: &AppState<Tg, Us, A, Ns>) -> Self {
        input.tg_update_service.clone()
    }
}

impl<Tg, Us, A, Ns> FromRef<AppState<Tg, Us, A, Ns>> for Arc<NudgeService<A, Ns>>
where
    Tg: TelegramClient,
    Us: UserStore,
    A: LlmAdapter,
    Ns: NudgeStore,
{
    fn from_ref(input: &AppState<Tg, Us, A, Ns>) -> Self {
        Arc::clone(&input.nudge_service)
    }
}

impl<Tg, Us, A, Ns> FromRef<AppState<Tg, Us, A, Ns>> for ManualNudgeMySqlLimiter
where
    Tg: TelegramClient,
    Us: UserStore,
    A: LlmAdapter,
    Ns: NudgeStore,
{
    fn from_ref(input: &AppState<Tg, Us, A, Ns>) -> Self {
        input.nudge_limiter.clone()
    }
}
