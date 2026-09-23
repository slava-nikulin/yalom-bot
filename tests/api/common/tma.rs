use hkdf::hmac::{Hmac, KeyInit, Mac};
use rustigram_miniapp::WebAppUser;
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub fn signed_init_data(bot_token: &str, user: &WebAppUser, auth_date: i64) -> String {
    let user_json = serde_json::to_string(user).expect("WebAppUser must be serializable");

    let mut entries = [
        ("auth_date", auth_date.to_string()),
        ("user", user_json.clone()),
    ];

    entries.sort_by_key(|(key, _)| *key);

    let data_check_string = entries
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("\n");

    let mut mac =
        HmacSha256::new_from_slice(b"WebAppData").expect("HMAC accepts keys of arbitrary size");

    mac.update(bot_token.as_bytes());

    let secret_key = mac.finalize().into_bytes();

    let mut mac =
        HmacSha256::new_from_slice(&secret_key).expect("HMAC accepts keys of arbitrary size");

    mac.update(data_check_string.as_bytes());

    let hash = hex::encode(mac.finalize().into_bytes());

    form_urlencoded::Serializer::new(String::new())
        .append_pair("auth_date", &auth_date.to_string())
        .append_pair("user", &user_json)
        .append_pair("hash", &hash)
        .finish()
}

pub fn test_user(id: i64) -> WebAppUser {
    WebAppUser {
        id,
        is_bot: None,
        first_name: "Test".into(),
        last_name: None,
        username: None,
        language_code: None,
        is_premium: None,
        added_to_attachment_menu: None,
        allows_write_to_pm: None,
        photo_url: None,
    }
}
