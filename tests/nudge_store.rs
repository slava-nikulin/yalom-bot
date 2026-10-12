use sqlx::MySqlPool;
use yalom_bot::nudge::{
    model::{NudgeDayTime, NudgeIntent},
    store::{NudgeMySqlStore, NudgeStore},
};

#[sqlx::test]
async fn test_get_random_nudge(db: MySqlPool) {
    sqlx::query(
        r#"
        INSERT INTO curated_nudges (intent, daytime, text)
        VALUES
            ('Observation', 'Any', 'Общее наблюдение 1'),
            ('Observation', 'Any', 'Общее наблюдение 2'),
            ('Observation', 'Morning', 'Утреннее наблюдение'),
            ('Question', 'Any', 'Общий вопрос')
        "#,
    )
    .execute(&db)
    .await
    .unwrap();

    let store = NudgeMySqlStore::new(db);

    // Случайный выбор только среди Observation + Any.
    let text = store
        .get_random_nudge(NudgeIntent::Observation, NudgeDayTime::Any)
        .await
        .unwrap()
        .unwrap();

    assert!(matches!(
        text.as_str(),
        "Общее наблюдение 1" | "Общее наблюдение 2"
    ));

    // Фильтрация по времени суток.
    let text = store
        .get_random_nudge(NudgeIntent::Observation, NudgeDayTime::Morning)
        .await
        .unwrap();

    assert_eq!(text.as_deref(), Some("Утреннее наблюдение"));

    // Фильтрация по intent.
    let text = store
        .get_random_nudge(NudgeIntent::Question, NudgeDayTime::Any)
        .await
        .unwrap();

    assert_eq!(text.as_deref(), Some("Общий вопрос"));
}

#[sqlx::test]
async fn test_get_random_nudge_not_found(db: MySqlPool) {
    let store = NudgeMySqlStore::new(db);

    let text = store
        .get_random_nudge(NudgeIntent::Observation, NudgeDayTime::Any)
        .await
        .unwrap();

    assert!(text.is_none());
}
