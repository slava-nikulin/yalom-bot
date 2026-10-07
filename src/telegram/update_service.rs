use std::time::Duration;

use async_channel::Sender;

use crate::{
    telegram::TelegramClient,
    user::store::{
        RegisterOutcome::{self, Existing},
        UserStore,
    },
    worker_pool::WorkerPool,
};

#[derive(Debug, thiserror::Error)]
pub enum TgUpdateServiceError {
    #[error("update queue is closed")]
    QueueClosed,
}

#[derive(Debug)]
pub enum TgUpdateJob {
    Start { tg_user_id: i64, chat_id: i64 },
    Message { chat_id: i64, text: String },
}

const WELCOME_MESSAGE: &str = "Welcome! Your account has been registered.";
const ACTIVATING_MESSAGE: &str = "Your account has been activated.";

#[derive(Clone)]
pub struct TgUpdateService {
    sender: Sender<TgUpdateJob>,
}

impl TgUpdateService {
    pub fn new<Tg, Us>(
        tg_client: Tg,
        user_store: Us,
        worker_count: usize,
        timeout: u64,
    ) -> (Self, WorkerPool<TgUpdateJob>)
    where
        Tg: TelegramClient,
        Us: UserStore,
    {
        let pool: WorkerPool<TgUpdateJob> =
            WorkerPool::new(worker_count, Duration::from_secs(timeout), move |update| {
                process_job(update, tg_client.clone(), user_store.clone())
            });

        (
            Self {
                sender: pool.sender(),
            },
            pool,
        )
    }

    pub async fn enqueue(&self, update_job: TgUpdateJob) -> Result<(), TgUpdateServiceError> {
        self.sender
            .send(update_job)
            .await
            .map_err(|_| TgUpdateServiceError::QueueClosed)
    }
}

async fn process_job<Tg, Us>(
    update_job: TgUpdateJob,
    tg_client: Tg,
    user_store: Us,
) -> anyhow::Result<()>
where
    Tg: TelegramClient,
    Us: UserStore,
{
    let (msg, chat_id) = match update_job {
        TgUpdateJob::Start {
            tg_user_id,
            chat_id,
        } => {
            let register_result = user_store.register(tg_user_id, chat_id).await?;
            (
                match register_result {
                    RegisterOutcome::Created => WELCOME_MESSAGE.to_owned(),
                    Existing => ACTIVATING_MESSAGE.to_owned(),
                },
                chat_id,
            )
        }
        TgUpdateJob::Message { chat_id, text } => (format!("You said: {}", text), chat_id),
    };

    tg_client.send_message(chat_id.into(), msg).await?;

    Ok(())
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::user::{model::YalomUser, store::UserStoreError};
    use std::sync::{Arc, Mutex};

    pub struct TgCall {}

    #[derive(Clone)]
    pub struct TestBotClient {
        pub calls: Arc<Mutex<Vec<TgCall>>>,
    }

    impl TestBotClient {
        fn new() -> Self {
            Self {
                calls: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    impl TelegramClient for TestBotClient {
        async fn send_message(
            &self,
            _chat_id: rustigram_types::user::ChatId,
            _text: String,
        ) -> anyhow::Result<()> {
            self.calls.lock().unwrap().push(TgCall {});

            Ok(())
        }
    }

    pub enum UserStoreCall {
        Register { tg_user_id: i64, chat_id: i64 },
    }

    #[derive(Clone)]
    pub struct TestUserStore {
        pub calls: Arc<Mutex<Vec<UserStoreCall>>>,
    }

    impl TestUserStore {
        fn new() -> Self {
            Self {
                calls: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    impl UserStore for TestUserStore {
        async fn register(
            &self,
            tg_user_id: i64,
            chat_id: i64,
        ) -> Result<RegisterOutcome, UserStoreError> {
            self.calls.lock().unwrap().push(UserStoreCall::Register {
                tg_user_id,
                chat_id,
            });
            Ok(RegisterOutcome::Created)
        }

        async fn get(&self, tg_user_id: i64) -> Result<YalomUser, UserStoreError> {
            Ok(YalomUser {
                tg_user_id,
                chat_id: 1,
                is_active: true,
            })
        }

        async fn update_bot_settings(
            &self,
            _tg_user_id: i64,
            _active: bool,
        ) -> Result<(), UserStoreError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn success_start() {
        let tg_bot_client = TestBotClient::new();
        let tg_calls = tg_bot_client.calls.clone();
        let user_store = TestUserStore::new();
        let store_calls = user_store.calls.clone();

        let (service, wp) = TgUpdateService::new(tg_bot_client, user_store, 1, 1);

        service
            .enqueue(TgUpdateJob::Start {
                tg_user_id: 1,
                chat_id: 2,
            })
            .await
            .unwrap();

        wp.shutdown().await;

        assert_eq!(tg_calls.lock().unwrap().len(), 1);
        assert_eq!(store_calls.lock().unwrap().len(), 1);
        assert!(matches!(
            store_calls.lock().unwrap().first().unwrap(),
            UserStoreCall::Register {
                tg_user_id: 1,
                chat_id: 2,
            },
        ));
    }
}
