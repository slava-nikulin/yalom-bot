use std::sync::Arc;

use axum::extract::FromRef;

use crate::{
    security::session::Sessions,
    telegram::{TelegramClient, update_service::TgUpdateService},
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

#[derive(Clone)]
pub struct AppState<Tg, Us> {
    pub telegram: TelegramState<Tg>,
    pub sessions: SessionState,
    pub user_store: UserStoreState<Us>,
    pub tg_bot_token: TgBotToken,
    pub tg_webhook_secret: TgWebhookSecret,
    pub tg_update_service: TgUpdateService,
}

impl<Tg: TelegramClient, Us: UserStore> AppState<Tg, Us> {
    pub fn new(
        tg_bot_token: &str,
        tg_webhook_secret: &str,
        sessions: Sessions,
        tg_bot: Tg,
        user_store: Us,
        tg_update_service: TgUpdateService,
    ) -> Self {
        Self {
            telegram: TelegramState::<Tg>(tg_bot),
            user_store: UserStoreState::<Us>(user_store),
            sessions: SessionState(Arc::new(sessions)),
            tg_bot_token: TgBotToken(Arc::from(tg_bot_token)),
            tg_webhook_secret: TgWebhookSecret(Arc::from(tg_webhook_secret)),
            tg_update_service,
        }
    }
}

impl<Tg, Us> FromRef<AppState<Tg, Us>> for TelegramState<Tg>
where
    Tg: TelegramClient,
    Us: UserStore,
{
    fn from_ref(input: &AppState<Tg, Us>) -> Self {
        input.telegram.clone()
    }
}

impl<Tg, Us> FromRef<AppState<Tg, Us>> for UserStoreState<Us>
where
    Tg: TelegramClient,
    Us: UserStore,
{
    fn from_ref(input: &AppState<Tg, Us>) -> Self {
        input.user_store.clone()
    }
}

impl<Tg, Us> FromRef<AppState<Tg, Us>> for SessionState
where
    Tg: TelegramClient,
    Us: UserStore,
{
    fn from_ref(input: &AppState<Tg, Us>) -> Self {
        input.sessions.clone()
    }
}

impl<Tg, Us> FromRef<AppState<Tg, Us>> for TgBotToken
where
    Tg: TelegramClient,
    Us: UserStore,
{
    fn from_ref(input: &AppState<Tg, Us>) -> Self {
        input.tg_bot_token.clone()
    }
}

impl<Tg, Us> FromRef<AppState<Tg, Us>> for TgWebhookSecret
where
    Tg: TelegramClient,
    Us: UserStore,
{
    fn from_ref(input: &AppState<Tg, Us>) -> Self {
        input.tg_webhook_secret.clone()
    }
}

impl<Tg, Us> FromRef<AppState<Tg, Us>> for TgUpdateService
where
    Tg: TelegramClient,
    Us: UserStore,
{
    fn from_ref(input: &AppState<Tg, Us>) -> Self {
        input.tg_update_service.clone()
    }
}
