use rustigram_api::BotClient;

#[derive(Clone)]
pub struct AppState<Tg> {
    pub telegram: Tg,
}

impl<Tg> AppState<Tg> {
    pub fn new(telegram: Tg) -> Self {
        Self { telegram }
    }
}

pub trait TelegramClient: Clone + Send + Sync + 'static {
    fn send_message(
        &self,
        chat_id: rustigram_types::user::ChatId,
        text: String,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
}

impl TelegramClient for BotClient {
    async fn send_message(
        &self,
        chat_id: rustigram_types::user::ChatId,
        text: String,
    ) -> anyhow::Result<()> {
        BotClient::send_message(self, chat_id, text).await?;
        Ok(())
    }
}
