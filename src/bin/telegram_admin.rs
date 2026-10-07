use rustigram_api::BotClient;
use rustigram_types::{MenuButton, WebAppInfo};
use yalom_bot::config::Config;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cfg = Config::load()?;
    let webhook_secret = cfg.telegram.webhook.secret_token;

    anyhow::ensure!(
        webhook_secret.len() >= 32,
        "telegram webhook secret must be at least 32 characters"
    );

    let bot = BotClient::from_token(cfg.telegram.token)?;

    bot.set_webhook(cfg.telegram.webhook.url.clone())
        .secret_token(webhook_secret)
        .await?;

    tracing::info!(
        webhook_url = %cfg.telegram.webhook.url,
        "telegram webhook configured"
    );

    bot.set_chat_menu_button()
        .menu_button(MenuButton::WebApp {
            text: "Menu".to_owned(),
            web_app: WebAppInfo {
                url: cfg.telegram.miniapp.url.to_string(),
            },
        })
        .await?;

    tracing::info!(
        miniapp_url = %cfg.telegram.miniapp.url,
        "telegram mini app menu button configured"
    );

    Ok(())
}
