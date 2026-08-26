use config::Environment;
use serde::Deserialize;
use std::net::IpAddr;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub telegram: TelegramConfig,
    pub http: HttpConfig,
}

#[derive(Debug, Deserialize)]
pub struct TelegramConfig {
    pub token: String,
    pub webhook: WebhookConfig,
}

#[derive(Debug, Deserialize)]
pub struct WebhookConfig {
    pub url: url::Url,
    pub secret_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct HttpConfig {
    pub host: IpAddr,
    pub port: u16,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        #[cfg(feature = "local-env")]
        dotenvy::dotenv()?;

        Ok(config::Config::builder()
            .set_default("http.host", "0.0.0.0")?
            .set_default("http.port", 3000)?
            .add_source(
                Environment::with_prefix("YALOM")
                    .prefix_separator("_")
                    .separator("__")
                    .try_parsing(true)
                    .ignore_empty(true),
            )
            .build()?
            .try_deserialize()?)
    }
}
