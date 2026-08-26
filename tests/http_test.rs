use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode},
};
use rustigram_api::BotClient;
use rustigram_types::{Message, Update, UpdateKind};
use tower::util::ServiceExt;
use yalom_bot::{app_state::AppState, http::app_router};

const TG_BOT_SECRET_TOKEN: &str = "123456789:ABC-DEF1234ghIkl-zyx57W2v1u123ew11";
const TG_WEBHOOK_SECRET_TOKEN: &str = "tg_webhook_secret_token";

fn test_router() -> Router {
    let tg_bot = BotClient::from_token(TG_BOT_SECRET_TOKEN).unwrap();
    let app_state = AppState::new(tg_bot);

    app_router(TG_WEBHOOK_SECRET_TOKEN.into(), app_state)
}

#[tokio::test]
async fn test_health() {
    let router = test_router();

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
    let router = test_router();

    let mut message = Message::default();
    message.message_id = 1;
    message.date = 0;
    message.chat.id = 123;

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
}

#[tokio::test]
async fn test_webhook_invalid_token() {
    let router = test_router();

    let mut message = Message::default();
    message.message_id = 1;
    message.date = 0;
    message.chat.id = 123;

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
}

#[tokio::test]
async fn test_webhook_valid_token_invalid_body() {
    let router = test_router();

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
}
