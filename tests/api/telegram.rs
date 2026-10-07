use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use rustigram_types::{Message, MessageEntity, MessageEntityKind, Update, UpdateKind, User};
use sqlx::mysql::MySqlPoolOptions;
use tower::util::ServiceExt;
use yalom_bot::user::model::YalomUser;

use crate::common::{TG_WEBHOOK_SECRET_TOKEN, TestApp};

#[tokio::test]
async fn test_health() {
    let app = TestApp::new(
        MySqlPoolOptions::new()
            .connect_lazy("mysql://root:root@localhost/test")
            .unwrap(),
    )
    .await
    .unwrap();

    let response = app
        .router
        .oneshot(
            Request::builder()
                .uri("/health")
                .method(Method::GET)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[sqlx::test]
async fn test_webhook_valid_token_registration(db: sqlx::MySqlPool) {
    let app = TestApp::new(db.clone()).await.unwrap();

    let mut message = Message::default();
    message.message_id = 1;
    message.date = 0;
    message.chat.id = 123;
    message.text = Some("/start".to_string());
    let mut user = User::default();
    user.id = 42;
    message.from = Some(user);
    message.entities = Some(vec![MessageEntity {
        kind: MessageEntityKind::BotCommand,
        offset: 0,
        length: 6, // "/start"
        url: None,
        user: None,
        language: None,
        custom_emoji_id: None,
        unix_time: None,
        date_time_format: None,
    }]);

    let update = Update {
        update_id: 1,
        kind: UpdateKind::Message(message),
    };
    let body = serde_json::to_vec(&update).unwrap();

    let response = app
        .router
        .oneshot(
            Request::builder()
                .uri("/tg/webhook")
                .method(Method::POST)
                .header("content-type", "application/json")
                .header("x-telegram-bot-api-secret-token", TG_WEBHOOK_SECRET_TOKEN)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    app.tg_update_job_wp.shutdown().await;
    assert_eq!(app.tg_calls.lock().unwrap().len(), 1);

    let user = sqlx::query_as::<_, YalomUser>(
        r#"
            SELECT
                tg_user_id,
                chat_id,
                is_active
            FROM users
            WHERE tg_user_id = ?
            "#,
    )
    .bind(42)
    .fetch_one(&db)
    .await
    .unwrap();

    assert_eq!(user.chat_id, 123);
    assert_eq!(user.tg_user_id, 42);
    assert!(user.is_active);
}

#[tokio::test]
async fn test_webhook_invalid_token() {
    let app = TestApp::new(
        MySqlPoolOptions::new()
            .connect_lazy("mysql://root:root@localhost/test")
            .unwrap(),
    )
    .await
    .unwrap();

    let mut message = Message::default();
    message.message_id = 1;
    message.date = 0;
    message.chat.id = 123;
    message.text = Some("foo".to_string());

    let update = Update {
        update_id: 1,
        kind: UpdateKind::Message(message),
    };
    let body = serde_json::to_vec(&update).unwrap();

    let response = app
        .router
        .oneshot(
            Request::builder()
                .uri("/tg/webhook")
                .method(Method::POST)
                .header("content-type", "application/json")
                .header("x-telegram-bot-api-secret-token", "invalid_token")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    app.tg_update_job_wp.shutdown().await;
    assert_eq!(app.tg_calls.lock().unwrap().len(), 0);
}

#[tokio::test]
async fn test_webhook_valid_token_invalid_body() {
    let app = TestApp::new(
        MySqlPoolOptions::new()
            .connect_lazy("mysql://root:root@localhost/test")
            .unwrap(),
    )
    .await
    .unwrap();

    let response = app
        .router
        .oneshot(
            Request::builder()
                .uri("/tg/webhook")
                .method(Method::POST)
                .header("content-type", "application/json")
                .header("x-telegram-bot-api-secret-token", TG_WEBHOOK_SECRET_TOKEN)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    app.tg_update_job_wp.shutdown().await;
    assert_eq!(app.tg_calls.lock().unwrap().len(), 0);
}
