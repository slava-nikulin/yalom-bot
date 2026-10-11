use sqlx::{Error, MySqlPool};

use crate::nudge::model::{Nudge, NudgeDayTime, NudgeIntent};

pub trait NudgeStore: Clone + Send + Sync + 'static {
    fn get_random_nudge(
        &self,
        intent: NudgeIntent,
        daytime: Option<NudgeDayTime>,
    ) -> impl std::future::Future<Output = Result<Option<Nudge>, Error>> + Send;
}

#[derive(Clone)]
pub struct NudgeMySqlStore {
    pool: MySqlPool,
}

impl NudgeMySqlStore {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

impl NudgeStore for NudgeMySqlStore {
    async fn get_random_nudge(
        &self,
        intent: NudgeIntent,
        daytime: Option<NudgeDayTime>,
    ) -> Result<Option<Nudge>, Error> {
        sqlx::query_as::<_, Nudge>(
            r#"
        SELECT id, text
        FROM curated_nudges
        WHERE intent = ?
          AND daytime = ?
        ORDER BY RAND()
        LIMIT 1
        "#,
        )
        .bind(intent)
        .bind(daytime.unwrap_or(NudgeDayTime::Any))
        .fetch_optional(&self.pool)
        .await
    }
}
