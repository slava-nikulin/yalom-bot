use axum::{
    body::Body,
    http::{Method, Request, StatusCode, header},
};
use tower::ServiceExt;
use tower_cookies::{Cookie, cookie::SameSite};
use yalom_bot::{
    http::miniapp::SESSION_COOKIE_NAME,
    security::session::{SESSION_TTL_SECONDS, SessionTokens},
};

use crate::common::{
    TG_TOKEN, TestApp,
    helper::now_secs,
    tma::{signed_init_data, test_user},
};

#[tokio::test]
async fn test_session_issue_missing_init_data() {
    let app = TestApp::new();

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
    let app = TestApp::new();

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
    let app = TestApp::new();

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
    let app = TestApp::new();

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
    let app = TestApp::new();

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
    assert_eq!(cookie.same_site(), Some(SameSite::Lax));
    assert_eq!(cookie.path(), Some("/"));
    assert_eq!(
        cookie.max_age().map(|d| d.whole_seconds()),
        Some(SESSION_TTL_SECONDS),
    );
}

#[tokio::test]
async fn test_menu_unauthorized_missing_cookie() {
    let app = TestApp::new();

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
    let app = TestApp::new();

    let session_tokens = SessionTokens::new("wrong key");
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

#[tokio::test]
#[should_panic(expected = "not yet implemented")]
async fn test_menu_happy_path_unimplemented() {
    let app = TestApp::new();
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

    let _response = router
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

    // assert_eq!(response.status(), StatusCode::OK);
}
