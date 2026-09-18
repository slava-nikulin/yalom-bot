use std::sync::{Arc, Mutex};

use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode},
};
use rustigram_types::{Message, Update, UpdateKind};
use tower::util::ServiceExt;
use yalom_bot::{
    app_state::{AppState, session_state::MiniAppState},
    http::app_router,
    security::session::SessionTokens,
    telegram::TelegramClient,
};

const TG_WEBHOOK_SECRET_TOKEN: &str = "tg_webhook_secret_token";
const TG_TOKEN: &str = "123456789:ABCdefGhIJKlmNoPQRsTUVwxYZ";

struct TgCall {}

#[derive(Clone)]
struct TestBotClient {
    calls: Arc<Mutex<Vec<TgCall>>>,
}

impl TestBotClient {
    fn new() -> Self {
        TestBotClient {
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
        let mut calls = self.calls.lock().unwrap();
        calls.push(TgCall {});

        Ok(())
    }
}

fn test_router() -> (Router, Arc<Mutex<Vec<TgCall>>>) {
    let tg_bot_client = TestBotClient::new();
    let calls = tg_bot_client.calls.clone();
    let app_state = AppState::new(tg_bot_client);
    let miniapp_state = MiniAppState::new(SessionTokens::new(TG_TOKEN).unwrap());

    (
        app_router(
            TG_WEBHOOK_SECRET_TOKEN.into(),
            TG_TOKEN,
            app_state,
            miniapp_state,
        ),
        calls,
    )
}

#[tokio::test]
async fn test_health() {
    let (router, _) = test_router();

    let response = router
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
    let (router, calls) = test_router();

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

    let response = router
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
    assert_eq!(calls.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn test_webhook_invalid_token() {
    let (router, calls) = test_router();

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

    let response = router
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
    assert_eq!(calls.lock().unwrap().len(), 0);
}

#[tokio::test]
async fn test_webhook_valid_token_invalid_body() {
    let (router, calls) = test_router();

    let response = router
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
    assert_eq!(calls.lock().unwrap().len(), 0);
}
