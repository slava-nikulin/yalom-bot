use anyhow::Context;
use rustigram_api::BotClient;
use rustigram_types::{MenuButton, WebAppInfo};
use yalom_bot::config::Config;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let cfg = Config::load()?;
    let webhook_secret = cfg
        .telegram
        .webhook
        .secret_token
        .context("telegram webhook secret token is required")?;

    let bot = BotClient::from_token(cfg.telegram.token)?;
    bot.set_webhook(cfg.telegram.webhook.url.clone())
        .secret_token(webhook_secret)
        .await?;
    bot.set_chat_menu_button()
        .menu_button(MenuButton::WebApp {
            text: "Menu".to_owned(),
            web_app: WebAppInfo {
                url: cfg.telegram.miniapp.url.to_string(),
            },
        })
        .await?;

    Ok(())
}
