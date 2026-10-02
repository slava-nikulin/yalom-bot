use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use rustigram_types::{Message, Update, UpdateKind};
use tower::util::ServiceExt;

use crate::common::{TG_WEBHOOK_SECRET_TOKEN, TestApp};

#[tokio::test]
async fn test_health() {
    let app = TestApp::new();

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

#[tokio::test]
async fn test_webhook_valid_token() {
    let app = TestApp::new();

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
                .header("x-telegram-bot-api-secret-token", TG_WEBHOOK_SECRET_TOKEN)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(app.tg_calls.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn test_webhook_invalid_token() {
    let app = TestApp::new();

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
    assert_eq!(app.tg_calls.lock().unwrap().len(), 0);
}

#[tokio::test]
async fn test_webhook_valid_token_invalid_body() {
    let app = TestApp::new();

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
    assert_eq!(app.tg_calls.lock().unwrap().len(), 0);
}
