use serde::Deserialize;

use crate::config::load;

#[derive(Deserialize)]
pub struct TelegramSetupConfig {
    pub telegram: TelegramSetupTelegramConfig,
}

#[derive(Deserialize)]
pub struct TelegramSetupTelegramConfig {
    pub token: String,
    pub webhook: WebhookConfig,
    pub miniapp: TelegramSetupMiniAppConfig,
}

#[derive(Deserialize)]
pub struct TelegramSetupMiniAppConfig {
    pub url: url::Url,
}

#[derive(Debug, Deserialize)]
pub struct WebhookConfig {
    pub url: url::Url,
    pub secret: String,
}

impl TelegramSetupConfig {
    pub fn load() -> anyhow::Result<Self> {
        load()
    }
}
