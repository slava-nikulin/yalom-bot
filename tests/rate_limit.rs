use std::time::Duration;

use chrono::{Duration as ChronoDuration, NaiveDateTime};
use sqlx::MySqlPool;
use tokio::task::JoinSet;

use yalom_bot::nudge::rate_limit::{ManualNudgeMySqlLimiter, RateLimitDecision, RateLimitReason};

// ---------- Helpers ----------

#[derive(Debug, PartialEq, Eq, sqlx::FromRow)]
struct RateLimitState {
    tat_utc: Option<NaiveDateTime>,
    window_start_utc: NaiveDateTime,
    request_count: u32,
}

async fn register_user(db: &MySqlPool, tg_user_id: i64) {
    sqlx::query(
        r#"
        INSERT INTO users (tg_user_id, chat_id)
        VALUES (?, ?)
        "#,
    )
    .bind(tg_user_id)
    .bind(tg_user_id)
    .execute(db)
    .await
    .unwrap();
}

async fn get_state(db: &MySqlPool, tg_user_id: i64) -> RateLimitState {
    sqlx::query_as::<_, RateLimitState>(
        r#"
        SELECT tat_utc, window_start_utc, request_count
        FROM manual_nudge_rate_limits
        WHERE tg_user_id = ?
        "#,
    )
    .bind(tg_user_id)
    .fetch_one(db)
    .await
    .unwrap()
}

async fn expire_cooldown(db: &MySqlPool, tg_user_id: i64) {
    sqlx::query(
        r#"
        UPDATE manual_nudge_rate_limits
        SET tat_utc = UTC_TIMESTAMP(6) - INTERVAL 1 SECOND
        WHERE tg_user_id = ?
        "#,
    )
    .bind(tg_user_id)
    .execute(db)
    .await
    .unwrap();
}

// ---------- Tests ----------

#[sqlx::test]
async fn first_request_initializes_state(db: MySqlPool) {
    register_user(&db, 42).await;

    let limiter = ManualNudgeMySqlLimiter::new(db.clone());

    let result = limiter.acquire(42).await.unwrap();

    assert!(matches!(result, RateLimitDecision::Allowed));

    let state = get_state(&db, 42).await;

    assert_eq!(state.request_count, 1);
    assert!(state.tat_utc.is_some());
    assert!(state.tat_utc.unwrap() > state.window_start_utc);
}

#[sqlx::test]
async fn immediate_second_request_is_rejected(db: MySqlPool) {
    register_user(&db, 42).await;

    let limiter = ManualNudgeMySqlLimiter::new(db.clone());

    assert!(matches!(
        limiter.acquire(42).await.unwrap(),
        RateLimitDecision::Allowed
    ));

    let before = get_state(&db, 42).await;

    let result = limiter.acquire(42).await.unwrap();

    match result {
        RateLimitDecision::Rejected {
            reason: RateLimitReason::Cooldown,
            retry_after,
        } => {
            assert!(retry_after > Duration::ZERO);
            assert!(retry_after <= Duration::from_secs(60));
        }

        other => panic!("expected cooldown, got {other:?}"),
    }

    // Отклонённый запрос не изменяет состояние.
    let after = get_state(&db, 42).await;

    assert_eq!(before, after);
}

#[sqlx::test]
async fn expired_cooldown_allows_next_request(db: MySqlPool) {
    register_user(&db, 42).await;

    let limiter = ManualNudgeMySqlLimiter::new(db.clone());

    assert!(matches!(
        limiter.acquire(42).await.unwrap(),
        RateLimitDecision::Allowed
    ));

    expire_cooldown(&db, 42).await;

    let before = get_state(&db, 42).await;

    let result = limiter.acquire(42).await.unwrap();

    assert!(matches!(result, RateLimitDecision::Allowed));

    let after = get_state(&db, 42).await;

    assert_eq!(after.request_count, 2);
    assert_eq!(after.window_start_utc, before.window_start_utc);

    // Новый cooldown установлен относительно текущего времени.
    assert!(after.tat_utc.unwrap() > before.tat_utc.unwrap());
}

#[sqlx::test]
async fn daily_quota_allows_exactly_20_requests(db: MySqlPool) {
    register_user(&db, 42).await;

    let limiter = ManualNudgeMySqlLimiter::new(db.clone());

    // Выполняем 20 разрешённых запросов.
    // Между ними искусственно завершаем cooldown.
    for request_number in 1..=20 {
        if request_number > 1 {
            expire_cooldown(&db, 42).await;
        }

        let result = limiter.acquire(42).await.unwrap();

        assert!(
            matches!(result, RateLimitDecision::Allowed),
            "request #{request_number} should be allowed"
        );
    }

    let state = get_state(&db, 42).await;
    assert_eq!(state.request_count, 20);

    // Исключаем влияние минутного ограничения.
    expire_cooldown(&db, 42).await;

    let before = get_state(&db, 42).await;

    // 21-й запрос должен быть отклонён.
    let result = limiter.acquire(42).await.unwrap();

    match result {
        RateLimitDecision::Rejected {
            reason: RateLimitReason::DailyQuota,
            retry_after,
        } => {
            assert!(retry_after > Duration::ZERO);
            assert!(retry_after <= Duration::from_secs(24 * 60 * 60));
        }

        other => panic!("expected daily quota rejection, got {other:?}"),
    }

    // Отказ не увеличивает request_count.
    assert_eq!(get_state(&db, 42).await, before);
}

#[sqlx::test]
async fn expired_window_resets_counter(db: MySqlPool) {
    register_user(&db, 42).await;

    let limiter = ManualNudgeMySqlLimiter::new(db.clone());

    limiter.acquire(42).await.unwrap();

    // Окно закончилось час назад.
    // Квота исчерпана, но это относится к старому окну.
    sqlx::query(
        r#"
        UPDATE manual_nudge_rate_limits
        SET
            window_start_utc = UTC_TIMESTAMP(6) - INTERVAL 25 HOUR,
            request_count = 20,
            tat_utc = UTC_TIMESTAMP(6) - INTERVAL 1 SECOND
        WHERE tg_user_id = ?
        "#,
    )
    .bind(42_i64)
    .execute(&db)
    .await
    .unwrap();

    let before = get_state(&db, 42).await;

    let result = limiter.acquire(42).await.unwrap();

    assert!(matches!(result, RateLimitDecision::Allowed));

    let after = get_state(&db, 42).await;

    // Началось новое окно, и запрос стал первым в нём.
    assert_eq!(after.request_count, 1);

    // Привязка окна сохраняется.
    // Оно сдвигается ровно на сутки, а не на момент запроса.
    assert_eq!(
        after.window_start_utc,
        before.window_start_utc + ChronoDuration::days(1)
    );
}

#[sqlx::test]
async fn window_skips_multiple_expired_periods(db: MySqlPool) {
    register_user(&db, 42).await;

    let limiter = ManualNudgeMySqlLimiter::new(db.clone());

    limiter.acquire(42).await.unwrap();

    // Пользователь не обращался 100 суток и 2 часа.
    sqlx::query(
        r#"
        UPDATE manual_nudge_rate_limits
        SET
            window_start_utc = DATE_SUB(
                DATE_SUB(UTC_TIMESTAMP(6), INTERVAL 100 DAY),
                INTERVAL 2 HOUR
            ),
            request_count = 20,
            tat_utc = UTC_TIMESTAMP(6) - INTERVAL 1 SECOND
        WHERE tg_user_id = ?
        "#,
    )
    .bind(42_i64)
    .execute(&db)
    .await
    .unwrap();

    let before = get_state(&db, 42).await;

    let result = limiter.acquire(42).await.unwrap();

    assert!(matches!(result, RateLimitDecision::Allowed));

    let after = get_state(&db, 42).await;

    assert_eq!(after.request_count, 1);

    // Сдвигаемся сразу на 100 периодов.
    assert_eq!(
        after.window_start_utc,
        before.window_start_utc + ChronoDuration::days(100)
    );
}

#[sqlx::test]
async fn rejected_request_does_not_advance_expired_window(db: MySqlPool) {
    register_user(&db, 42).await;

    let limiter = ManualNudgeMySqlLimiter::new(db.clone());

    limiter.acquire(42).await.unwrap();

    // Суточное окно уже закончилось,
    // но cooldown всё ещё действует.
    sqlx::query(
        r#"
        UPDATE manual_nudge_rate_limits
        SET
            window_start_utc = UTC_TIMESTAMP(6) - INTERVAL 25 HOUR,
            request_count = 20,
            tat_utc = UTC_TIMESTAMP(6) + INTERVAL 30 SECOND
        WHERE tg_user_id = ?
        "#,
    )
    .bind(42_i64)
    .execute(&db)
    .await
    .unwrap();

    let before = get_state(&db, 42).await;

    let result = limiter.acquire(42).await.unwrap();

    assert!(matches!(
        result,
        RateLimitDecision::Rejected {
            reason: RateLimitReason::Cooldown,
            ..
        }
    ));

    // Хотя окно логически обновилось, из-за отказа
    // транзакция не должна записывать новое состояние.
    assert_eq!(get_state(&db, 42).await, before);

    // После истечения cooldown запрос проходит.
    expire_cooldown(&db, 42).await;

    assert!(matches!(
        limiter.acquire(42).await.unwrap(),
        RateLimitDecision::Allowed
    ));

    let after = get_state(&db, 42).await;

    assert_eq!(after.request_count, 1);
    assert_eq!(
        after.window_start_utc,
        before.window_start_utc + ChronoDuration::days(1)
    );
}

#[sqlx::test]
async fn retry_after_accounts_for_both_limits(db: MySqlPool) {
    register_user(&db, 42).await;

    let limiter = ManualNudgeMySqlLimiter::new(db.clone());

    limiter.acquire(42).await.unwrap();

    // До конца суточного окна — 1 час.
    // До завершения cooldown — 2 часа.
    sqlx::query(
        r#"
        UPDATE manual_nudge_rate_limits
        SET
            window_start_utc = UTC_TIMESTAMP(6) - INTERVAL 23 HOUR,
            request_count = 20,
            tat_utc = UTC_TIMESTAMP(6) + INTERVAL 2 HOUR
        WHERE tg_user_id = ?
        "#,
    )
    .bind(42_i64)
    .execute(&db)
    .await
    .unwrap();

    let before = get_state(&db, 42).await;

    let result = limiter.acquire(42).await.unwrap();

    match result {
        RateLimitDecision::Rejected {
            reason: RateLimitReason::DailyQuota,
            retry_after,
        } => {
            // Должны ждать завершения ОБОИХ ограничений,
            // следовательно, примерно 2 часа, а не один.
            assert!(retry_after > Duration::from_secs(90 * 60));
            assert!(retry_after <= Duration::from_secs(2 * 60 * 60));
        }

        other => panic!("expected daily quota rejection, got {other:?}"),
    }

    assert_eq!(get_state(&db, 42).await, before);
}

#[sqlx::test]
async fn limits_are_independent_per_user(db: MySqlPool) {
    register_user(&db, 42).await;
    register_user(&db, 43).await;

    let limiter = ManualNudgeMySqlLimiter::new(db.clone());

    assert!(matches!(
        limiter.acquire(42).await.unwrap(),
        RateLimitDecision::Allowed
    ));

    // Второй пользователь имеет собственную квоту.
    assert!(matches!(
        limiter.acquire(43).await.unwrap(),
        RateLimitDecision::Allowed
    ));

    // Первый пользователь по-прежнему ограничен.
    assert!(matches!(
        limiter.acquire(42).await.unwrap(),
        RateLimitDecision::Rejected {
            reason: RateLimitReason::Cooldown,
            ..
        }
    ));

    assert_eq!(get_state(&db, 42).await.request_count, 1);
    assert_eq!(get_state(&db, 43).await.request_count, 1);
}

#[sqlx::test]
async fn limit_survives_limiter_recreation(db: MySqlPool) {
    register_user(&db, 42).await;

    let first = ManualNudgeMySqlLimiter::new(db.clone());

    assert!(matches!(
        first.acquire(42).await.unwrap(),
        RateLimitDecision::Allowed
    ));

    // Создаём новый экземпляр с тем же MySQL.
    let second = ManualNudgeMySqlLimiter::new(db.clone());

    let result = second.acquire(42).await.unwrap();

    assert!(matches!(
        result,
        RateLimitDecision::Rejected {
            reason: RateLimitReason::Cooldown,
            ..
        }
    ));

    assert_eq!(get_state(&db, 42).await.request_count, 1);
}

#[sqlx::test]
async fn concurrent_first_requests_allow_only_one(db: MySqlPool) {
    register_user(&db, 42).await;

    let limiter = ManualNudgeMySqlLimiter::new(db.clone());

    let mut tasks = JoinSet::new();

    // У пользователя ещё нет строки в rate_limit.
    // Запускаем одновременно несколько acquire().
    for _ in 0..8 {
        let limiter = limiter.clone();

        tasks.spawn(async move { limiter.acquire(42).await.unwrap() });
    }

    let mut allowed = 0;
    let mut rejected = 0;

    while let Some(result) = tasks.join_next().await {
        match result.unwrap() {
            RateLimitDecision::Allowed => {
                allowed += 1;
            }

            RateLimitDecision::Rejected {
                reason: RateLimitReason::Cooldown,
                ..
            } => {
                rejected += 1;
            }

            other => panic!("unexpected rate limit result: {other:?}"),
        }
    }

    assert_eq!(allowed, 1);
    assert_eq!(rejected, 7);

    // Несмотря на конкурентные запросы, списана одна квота.
    let state = get_state(&db, 42).await;

    assert_eq!(state.request_count, 1);
}

#[sqlx::test]
async fn unregistered_user_cannot_acquire(db: MySqlPool) {
    let limiter = ManualNudgeMySqlLimiter::new(db.clone());

    // Пользователя нет в users, сработает FOREIGN KEY.
    let result = limiter.acquire(42).await;

    assert!(matches!(result, Err(sqlx::Error::Database(_))));

    // Не должно остаться строки rate_limit.
    let row = sqlx::query(
        r#"
        SELECT tg_user_id
        FROM manual_nudge_rate_limits
        WHERE tg_user_id = ?
        "#,
    )
    .bind(42_i64)
    .fetch_optional(&db)
    .await
    .unwrap();

    assert!(row.is_none());
}
