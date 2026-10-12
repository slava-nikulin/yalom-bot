use serde::Deserialize;

use crate::config::load;

#[derive(Deserialize)]
pub struct TelegramSetupConfig {
    pub telegram: TelegramSetupTelegramConfig,
}

#[derive(Deserialize)]
pub struct TelegramSetupTelegramConfig {
    pub token: String,
    pub webhook_url: url::Url,
    pub webhook_secret: String,
    pub miniapp_url: url::Url,
}

impl TelegramSetupConfig {
    pub fn load() -> anyhow::Result<Self> {
        load()
    }
}
