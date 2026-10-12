pub mod migrate_config;
pub mod tg_setup_config;

use config::Environment;
use serde::{Deserialize, de::DeserializeOwned};
use std::net::IpAddr;

#[derive(Deserialize)]
pub struct AppConfig {
    pub telegram: TelegramConfig,
    pub http: HttpConfig,
    pub database: DatabaseConfig,
    pub openai: OpenAiConfig,
}

#[derive(Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Deserialize)]
pub struct OpenAiConfig {
    pub api_key: String,
}

#[derive(Debug, Deserialize)]
pub struct TelegramConfig {
    pub token: String,
    pub miniapp_session_key: String,
    pub web_hook_secret: String,
}

#[derive(Debug, Deserialize)]
pub struct HttpConfig {
    pub host: IpAddr,
    pub port: u16,
}

impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        load()
    }
}

pub fn load<T: DeserializeOwned>() -> anyhow::Result<T> {
    Ok(config::Config::builder()
        .set_default("http.host", "0.0.0.0")?
        .set_default("http.port", 3000)?
        .add_source(
            Environment::with_prefix("YALOM")
                .prefix_separator("_")
                .separator("__")
                .ignore_empty(true),
        )
        .build()?
        .try_deserialize()?)
}
