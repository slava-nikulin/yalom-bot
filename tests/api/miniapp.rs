use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode, header},
};
use sqlx::mysql::MySqlPoolOptions;
use tower::ServiceExt;
use tower_cookies::{Cookie, cookie::SameSite};
use yalom_bot::{
    http::miniapp::{
        dto::{MenuState, UpdateMenuRequest},
        extractors::SESSION_COOKIE_NAME,
    },
    security::session::{SESSION_TTL_SECONDS, Sessions},
};

use crate::common::{
    TG_TOKEN, TestApp,
    helper::now_secs,
    tma::{signed_init_data, test_user},
};

#[tokio::test]
async fn test_session_issue_missing_init_data() {
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
                .uri("/api/miniapp/session")
                .method(Method::POST)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_session_issue_malformed_init_data() {
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
                .uri("/api/miniapp/session")
                .method(Method::POST)
                .header("X-Tma-Init-Data", "wrong data")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_session_issue_invalid_hmac() {
    let app = TestApp::new(
        MySqlPoolOptions::new()
            .connect_lazy("mysql://root:root@localhost/test")
            .unwrap(),
    )
    .await
    .unwrap();

    let init_data = signed_init_data("999999:wrong-bot-token", &test_user(42), now_secs());

    let response = app
        .router
        .oneshot(
            Request::builder()
                .uri("/api/miniapp/session")
                .method(Method::POST)
                .header("X-Tma-Init-Data", init_data)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_session_issue_expired_auth_date() {
    let app = TestApp::new(
        MySqlPoolOptions::new()
            .connect_lazy("mysql://root:root@localhost/test")
            .unwrap(),
    )
    .await
    .unwrap();

    let init_data = signed_init_data(
        TG_TOKEN,
        &test_user(42),
        now_secs() - SESSION_TTL_SECONDS - 1,
    );

    let response = app
        .router
        .oneshot(
            Request::builder()
                .uri("/api/miniapp/session")
                .method(Method::POST)
                .header("X-Tma-Init-Data", init_data)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_session_issue_happy_path() {
    let app = TestApp::new(
        MySqlPoolOptions::new()
            .connect_lazy("mysql://root:root@localhost/test")
            .unwrap(),
    )
    .await
    .unwrap();

    let init_data = signed_init_data(TG_TOKEN, &test_user(42), now_secs());

    let response = app
        .router
        .oneshot(
            Request::builder()
                .uri("/api/miniapp/session")
                .method(Method::POST)
                .header("X-Tma-Init-Data", init_data)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let cookie_header = response
        .headers()
        .get(header::SET_COOKIE)
        .expect("session endpoint should set a cookie")
        .to_str()
        .unwrap();

    let cookie = Cookie::parse(cookie_header.to_owned()).unwrap();

    assert_eq!(cookie.name(), SESSION_COOKIE_NAME);
    assert!(!cookie.value().is_empty());

    assert_eq!(cookie.http_only(), Some(true));
    assert_eq!(cookie.secure(), Some(true));
    assert_eq!(cookie.same_site(), Some(SameSite::None));
    assert_eq!(cookie.partitioned(), Some(true));
    assert_eq!(cookie.path(), Some("/"));
    assert_eq!(
        cookie.max_age().map(|d| d.whole_seconds()),
        Some(SESSION_TTL_SECONDS),
    );
}

#[tokio::test]
async fn test_menu_unauthorized_missing_cookie() {
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
                .uri("/api/miniapp/menu")
                .method(Method::GET)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_menu_unauthorized_cookie_wrong_token() {
    let app = TestApp::new(
        MySqlPoolOptions::new()
            .connect_lazy("mysql://root:root@localhost/test")
            .unwrap(),
    )
    .await
    .unwrap();

    let session_tokens = Sessions::new("wrong key");
    let issued_token = session_tokens.issue(123).unwrap();

    let cookie_header = format!("{}={}", SESSION_COOKIE_NAME, issued_token);
    let response = app
        .router
        .oneshot(
            Request::builder()
                .uri("/api/miniapp/menu")
                .method(Method::GET)
                .header(header::COOKIE, cookie_header)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn test_menu_user_not_found(db: sqlx::MySqlPool) {
    let app = TestApp::new(db.clone()).await.unwrap();
    let router = app.router;

    let init_data = signed_init_data(TG_TOKEN, &test_user(42), now_secs());

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/miniapp/session")
                .method(Method::POST)
                .header("X-Tma-Init-Data", init_data)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let set_cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .expect("session endpoint should set a cookie")
        .to_str()
        .unwrap();

    let cookie = Cookie::parse(set_cookie.to_owned()).unwrap();

    let cookie_header = format!("{}={}", cookie.name(), cookie.value());

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/miniapp/menu")
                .method(Method::GET)
                .header(header::COOKIE, cookie_header)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn test_menu_happy_path(db: sqlx::MySqlPool) {
    let app = TestApp::new(db.clone()).await.unwrap();

    sqlx::query(
        r#"
        INSERT INTO users (
            tg_user_id,
            chat_id,
            is_active
        )
        VALUES (?, ?, TRUE)
        "#,
    )
    .bind(42_i64)
    .bind(100_i64)
    .execute(&db)
    .await
    .unwrap();

    let router = app.router;

    let init_data = signed_init_data(TG_TOKEN, &test_user(42), now_secs());

    //get session token
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/miniapp/session")
                .method(Method::POST)
                .header("X-Tma-Init-Data", init_data)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // set session token in cookies
    let set_cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .expect("session endpoint should set a cookie")
        .to_str()
        .unwrap();

    let cookie = Cookie::parse(set_cookie.to_owned()).unwrap();

    let cookie_header = format!("{}={}", cookie.name(), cookie.value());

    // get menu
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/miniapp/menu")
                .method(Method::GET)
                .header(header::COOKIE, cookie_header.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();

    let menu: MenuState = serde_json::from_slice(&body).unwrap();

    assert!(!menu.paused);

    //update menu settings

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/miniapp/menu")
                .header("content-type", "application/json")
                .method(Method::PATCH)
                .header(header::COOKIE, cookie_header.clone())
                .body(Body::from(
                    serde_json::to_string(&UpdateMenuRequest { paused: true }).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();

    let menu: MenuState = serde_json::from_slice(&body).unwrap();

    assert!(menu.paused);

    // get menu again
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/miniapp/menu")
                .method(Method::GET)
                .header(header::COOKIE, cookie_header.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();

    let menu: MenuState = serde_json::from_slice(&body).unwrap();

    assert!(menu.paused);
}

#[sqlx::test]
async fn test_update_bot_settings_user_not_found(db: sqlx::MySqlPool) {
    let app = TestApp::new(db.clone()).await.unwrap();

    let router = app.router;

    let init_data = signed_init_data(TG_TOKEN, &test_user(42), now_secs());

    //get session token
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/miniapp/session")
                .method(Method::POST)
                .header("X-Tma-Init-Data", init_data)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // set session token in cookies
    let set_cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .expect("session endpoint should set a cookie")
        .to_str()
        .unwrap();

    let cookie = Cookie::parse(set_cookie.to_owned()).unwrap();

    let cookie_header = format!("{}={}", cookie.name(), cookie.value());

    //update menu settings

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/miniapp/menu")
                .header("content-type", "application/json")
                .method(Method::PATCH)
                .header(header::COOKIE, cookie_header.clone())
                .body(Body::from(
                    serde_json::to_string(&UpdateMenuRequest { paused: true }).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
