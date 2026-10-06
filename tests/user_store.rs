use yalom_bot::user::store::{MySqlUserStore, RegisterOutcome, UserStore, UserStoreError};

#[sqlx::test]
async fn test_register_insert(db: sqlx::MySqlPool) {
    let store = MySqlUserStore::new(db);

    store.register(1, 2).await.unwrap();

    let yalom_user = store.get(1).await.unwrap();

    assert_eq!(yalom_user.chat_id, 2);
    assert_eq!(yalom_user.tg_user_id, 1);
    assert!(yalom_user.is_active);
}

#[sqlx::test]
async fn test_register_update(db: sqlx::MySqlPool) {
    sqlx::query(
        r#"
        INSERT INTO users (
            tg_user_id,
            chat_id,
            is_active
        )
        VALUES (?, ?, FALSE)
        "#,
    )
    .bind(42_i64)
    .bind(100_i64)
    .execute(&db)
    .await
    .unwrap();

    let store = MySqlUserStore::new(db);
    let yalom_user1 = store.get(42).await.unwrap();
    assert_eq!(yalom_user1.chat_id, 100);
    assert_eq!(yalom_user1.tg_user_id, 42);
    assert!(!yalom_user1.is_active);

    let res = store.register(42, 200).await.unwrap();
    assert!(matches!(res, RegisterOutcome::Existing));
    let yalom_user2 = store.get(42).await.unwrap();
    assert_eq!(yalom_user2.chat_id, 200);

    let res = store.register(42, 300).await.unwrap();
    assert!(matches!(res, RegisterOutcome::Existing));

    let yalom_user3 = store.get(42).await.unwrap();
    assert_eq!(yalom_user3.chat_id, 300);
    assert_eq!(yalom_user3.tg_user_id, 42);
    assert!(yalom_user3.is_active);
}

#[sqlx::test]
async fn test_get_not_found(db: sqlx::MySqlPool) {
    let store = MySqlUserStore::new(db);

    let res = store.get(1).await;
    assert!(matches!(res, Err(UserStoreError::NotFound)));
}

#[sqlx::test]
async fn test_update_user_settings(db: sqlx::MySqlPool) {
    let store = MySqlUserStore::new(db);

    store.register(1, 2).await.unwrap();

    let yalom_user = store.get(1).await.unwrap();
    assert!(yalom_user.is_active);

    store.update_bot_settings(1, false).await.unwrap();

    let yalom_user = store.get(1).await.unwrap();
    assert!(!yalom_user.is_active);

    store.update_bot_settings(1, true).await.unwrap();

    let yalom_user = store.get(1).await.unwrap();
    assert!(yalom_user.is_active);
}

#[sqlx::test]
async fn test_update_user_settings_not_found(db: sqlx::MySqlPool) {
    let store = MySqlUserStore::new(db);

    let res = store.update_bot_settings(1, false).await;

    assert!(matches!(res, Err(UserStoreError::NotFound)));
}
