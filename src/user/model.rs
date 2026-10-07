use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub struct YalomUser {
    pub tg_user_id: i64,
    pub chat_id: i64,
    pub is_active: bool,
}
