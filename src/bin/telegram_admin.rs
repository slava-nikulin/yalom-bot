use anyhow::Context;
use rustigram_api::BotClient;
use yalom_bot::config::Config;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let cfg = Config::load()?;
    let webhook_secret = cfg
        .telegram
        .webhook
        .secret_token
        .context("telegram webhook secret token is required")?;

    BotClient::from_token(cfg.telegram.token)?
        .set_webhook(cfg.telegram.webhook.url.clone())
        .secret_token(webhook_secret)
        .await?;

    Ok(())
}
