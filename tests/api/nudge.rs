use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
    response::Response,
};
use chrono::{Timelike, Utc};
use serde_json::{Value, json};
use sqlx::MySqlPool;
use tower::ServiceExt;

use yalom_bot::{http::miniapp::extractors::SESSION_COOKIE_NAME, security::session::Sessions};

use crate::common::{FAKE_NUDGE_TEXT, SESSION_KEY, TestApp};

const USER_ID: i64 = 42;
const CHAT_ID: i64 = 100;

async fn register_user(db: &MySqlPool) {
    sqlx::query("INSERT INTO users (tg_user_id, chat_id) VALUES (?, ?)")
        .bind(USER_ID)
        .bind(CHAT_ID)
        .execute(db)
        .await
        .unwrap();
}

fn session_cookie() -> String {
    let token = Sessions::new(SESSION_KEY).issue(USER_ID).unwrap();

    format!("{SESSION_COOKIE_NAME}={token}")
}

async fn post_nudge(router: &Router, cookie: &str, timezone: &str) -> Response {
    router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/miniapp/nudge")
                .method(Method::POST)
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "timezone": timezone }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn response_json(response: Response) -> Value {
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();

    serde_json::from_slice(&body).unwrap()
}

async fn assert_api_error(response: Response, status: StatusCode, code: &str) -> Option<u64> {
    assert_eq!(response.status(), status);

    let retry_after = response
        .headers()
        .get(header::RETRY_AFTER)
        .map(|value| value.to_str().unwrap().parse::<u64>().unwrap());

    let body = response_json(response).await;

    assert_eq!(body["code"].as_str(), Some(code));
    assert_eq!(
        body.get("retry_after_seconds").and_then(Value::as_u64),
        retry_after,
    );

    retry_after
}

fn timezone_for(night: bool) -> String {
    let now = Utc::now();

    chrono_tz::TZ_VARIANTS
        .iter()
        .find(|tz| {
            let hour = now.with_timezone(*tz).hour();

            if night {
                (1..=4).contains(&hour)
            } else {
                (10..=18).contains(&hour)
            }
        })
        .expect("suitable timezone should exist")
        .to_string()
}

#[sqlx::test]
async fn test_generate_nudge_success(db: MySqlPool) {
    register_user(&db).await;

    let app = TestApp::new(db.clone()).await.unwrap();
    let timezone = timezone_for(false);

    let response = post_nudge(&app.router, &session_cookie(), &timezone).await;

    assert_eq!(response.status(), StatusCode::OK);

    let body = response_json(response).await;
    assert_eq!(body["text"], FAKE_NUDGE_TEXT);

    // Дожидаемся обработки Telegram job.
    app.tg_update_job_wp.shutdown().await;

    let tg_calls = app.tg_calls.lock().unwrap();

    assert_eq!(tg_calls.len(), 1);
    assert_eq!(tg_calls[0].chat_id, CHAT_ID.into());
    assert_eq!(tg_calls[0].text, FAKE_NUDGE_TEXT);

    let llm_calls = app.llm_calls.lock().unwrap();

    assert_eq!(llm_calls.len(), 1);
    assert!(llm_calls[0].context.contains(&timezone));

    let count: u32 = sqlx::query_scalar(
        "SELECT request_count FROM manual_nudge_rate_limits WHERE tg_user_id = ?",
    )
    .bind(USER_ID)
    .fetch_one(&db)
    .await
    .unwrap();

    assert_eq!(count, 1);
}

#[sqlx::test]
async fn test_generate_nudge_cooldown(db: MySqlPool) {
    register_user(&db).await;

    let app = TestApp::new(db).await.unwrap();
    let cookie = session_cookie();
    let timezone = timezone_for(false);

    let first = post_nudge(&app.router, &cookie, &timezone).await;
    assert_eq!(first.status(), StatusCode::OK);

    let second = post_nudge(&app.router, &cookie, &timezone).await;

    let retry_after = assert_api_error(second, StatusCode::TOO_MANY_REQUESTS, "rate_limited")
        .await
        .unwrap();

    assert!((1..=60).contains(&retry_after));

    app.tg_update_job_wp.shutdown().await;

    // Второй запрос не вызывает повторную генерацию и отправку.
    assert_eq!(app.llm_calls.lock().unwrap().len(), 1);
    assert_eq!(app.tg_calls.lock().unwrap().len(), 1);
}

#[sqlx::test]
async fn test_generate_nudge_daily_limit(db: MySqlPool) {
    register_user(&db).await;

    sqlx::query(
        r#"
        INSERT INTO manual_nudge_rate_limits (
            tg_user_id,
            tat_utc,
            window_start_utc,
            request_count
        )
        VALUES (
            ?,
            UTC_TIMESTAMP(6) - INTERVAL 1 SECOND,
            UTC_TIMESTAMP(6) - INTERVAL 1 HOUR,
            20
        )
        "#,
    )
    .bind(USER_ID)
    .execute(&db)
    .await
    .unwrap();

    let app = TestApp::new(db).await.unwrap();

    let response = post_nudge(&app.router, &session_cookie(), &timezone_for(false)).await;

    let retry_after = assert_api_error(response, StatusCode::TOO_MANY_REQUESTS, "daily_limit")
        .await
        .unwrap();

    assert!(retry_after > 0);
    assert!(retry_after <= 24 * 60 * 60);

    app.tg_update_job_wp.shutdown().await;

    assert!(app.llm_calls.lock().unwrap().is_empty());
    assert!(app.tg_calls.lock().unwrap().is_empty());
}

#[sqlx::test]
async fn test_generate_nudge_inappropriate_time(db: MySqlPool) {
    register_user(&db).await;

    let app = TestApp::new(db.clone()).await.unwrap();

    let response = post_nudge(&app.router, &session_cookie(), &timezone_for(true)).await;

    assert_api_error(
        response,
        StatusCode::UNPROCESSABLE_ENTITY,
        "inappropriate_time",
    )
    .await;

    // До limiter.acquire() выполнение не дошло.
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM manual_nudge_rate_limits WHERE tg_user_id = ?")
            .bind(USER_ID)
            .fetch_one(&db)
            .await
            .unwrap();

    assert_eq!(count, 0);

    app.tg_update_job_wp.shutdown().await;

    assert!(app.llm_calls.lock().unwrap().is_empty());
    assert!(app.tg_calls.lock().unwrap().is_empty());
}

#[sqlx::test]
async fn test_generate_nudge_invalid_timezone(db: MySqlPool) {
    register_user(&db).await;

    let app = TestApp::new(db).await.unwrap();

    let response = post_nudge(&app.router, &session_cookie(), "Invalid/Timezone").await;

    assert_api_error(response, StatusCode::BAD_REQUEST, "invalid_timezone").await;

    app.tg_update_job_wp.shutdown().await;

    assert!(app.llm_calls.lock().unwrap().is_empty());
    assert!(app.tg_calls.lock().unwrap().is_empty());
}

#[sqlx::test]
async fn test_generate_nudge_user_not_found(db: MySqlPool) {
    // Сессия валидна, но пользователя в MySQL нет.
    let app = TestApp::new(db).await.unwrap();

    let response = post_nudge(&app.router, &session_cookie(), &timezone_for(false)).await;

    assert_api_error(response, StatusCode::NOT_FOUND, "user_not_found").await;

    app.tg_update_job_wp.shutdown().await;

    assert!(app.llm_calls.lock().unwrap().is_empty());
    assert!(app.tg_calls.lock().unwrap().is_empty());
}
