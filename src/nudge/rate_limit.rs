use std::time::Duration;

use chrono::{Duration as ChronoDuration, NaiveDateTime};
use sqlx::{FromRow, MySqlPool};

const COOLDOWN_SECONDS: i64 = 60;
const WINDOW_SECONDS: i64 = 24 * 60 * 60;
const DAILY_LIMIT: u32 = 20;

#[derive(Debug)]
pub enum RateLimitReason {
    Cooldown,
    DailyQuota,
}

#[derive(Debug)]
pub enum RateLimitDecision {
    Allowed,
    Rejected {
        reason: RateLimitReason,
        retry_after: Duration,
    },
}

#[derive(FromRow)]
struct RateLimitRow {
    tat_utc: Option<NaiveDateTime>,
    window_start_utc: NaiveDateTime,
    request_count: u32,
}

#[derive(Clone)]
pub struct ManualNudgeMySqlLimiter {
    pool: MySqlPool,
}

impl ManualNudgeMySqlLimiter {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub async fn acquire(&self, tg_user_id: i64) -> Result<RateLimitDecision, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        // Создаём строку, если пользователь обращается впервые.
        // При конфликте PK ничего не меняем.
        sqlx::query(
            r#"
            INSERT INTO manual_nudge_rate_limits (
                tg_user_id,
                window_start_utc
            )
            VALUES (?, UTC_TIMESTAMP(6))
            ON DUPLICATE KEY UPDATE tg_user_id = tg_user_id
            "#,
        )
        .bind(tg_user_id)
        .execute(&mut *tx)
        .await?;

        // Блокируем состояние этого пользователя до конца транзакции.
        let row = sqlx::query_as::<_, RateLimitRow>(
            r#"
            SELECT
                tat_utc,
                window_start_utc,
                request_count
            FROM manual_nudge_rate_limits
            WHERE tg_user_id = ?
            FOR UPDATE
            "#,
        )
        .bind(tg_user_id)
        .fetch_one(&mut *tx)
        .await?;

        // Читаем время ПОСЛЕ получения блокировки.
        let now: NaiveDateTime = sqlx::query_scalar("SELECT UTC_TIMESTAMP(6)")
            .fetch_one(&mut *tx)
            .await?;

        // 1. Суточная квота: определяем актуальное окно.
        let elapsed = now.signed_duration_since(row.window_start_utc);
        let periods = elapsed.num_seconds() / WINDOW_SECONDS;

        let (window_start, count) = if periods > 0 {
            (row.window_start_utc + ChronoDuration::days(periods), 0)
        } else {
            (row.window_start_utc, row.request_count)
        };

        // 2. Проверяем оба ограничения, ничего пока не записывая.
        let daily_wait = (count >= DAILY_LIMIT)
            .then(|| window_start + ChronoDuration::seconds(WINDOW_SECONDS) - now);

        let cooldown_wait = row.tat_utc.filter(|&tat| tat > now).map(|tat| tat - now);

        if daily_wait.is_some() || cooldown_wait.is_some() {
            let reason = if daily_wait.is_some() {
                RateLimitReason::DailyQuota
            } else {
                RateLimitReason::Cooldown
            };

            // Следующая попытка должна удовлетворять обоим лимитам.
            let wait = daily_wait
                .unwrap_or_default()
                .max(cooldown_wait.unwrap_or_default());

            tx.rollback().await?;

            return Ok(RateLimitDecision::Rejected {
                reason,
                retry_after: wait.to_std().expect("retry duration must be non-negative"),
            });
        }

        // 3. Оба ограничения разрешили запрос.
        // GCRA с burst = 1: следующий запрос через 60 секунд.
        let next_tat = now + ChronoDuration::seconds(COOLDOWN_SECONDS);

        sqlx::query(
            r#"
            UPDATE manual_nudge_rate_limits
            SET
                tat_utc = ?,
                window_start_utc = ?,
                request_count = ?
            WHERE tg_user_id = ?
            "#,
        )
        .bind(next_tat)
        .bind(window_start)
        .bind(count + 1)
        .bind(tg_user_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(RateLimitDecision::Allowed)
    }
}
