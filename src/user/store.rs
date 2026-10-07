use sqlx::{MySqlPool, Pool};

use crate::user::model::YalomUser;

#[derive(Debug, thiserror::Error)]
pub enum UserStoreError {
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("user not found")]
    NotFound,
}

#[derive(Debug)]
pub enum RegisterOutcome {
    Created,
    Existing,
}

pub trait UserStore: Clone + Send + Sync + 'static {
    fn register(
        &self,
        tg_user_id: i64,
        chat_id: i64,
    ) -> impl Future<Output = Result<RegisterOutcome, UserStoreError>> + Send;

    fn get(
        &self,
        tg_user_id: i64,
    ) -> impl Future<Output = Result<YalomUser, UserStoreError>> + Send;

    fn update_bot_settings(
        &self,
        tg_user_id: i64,
        active: bool,
    ) -> impl Future<Output = Result<(), UserStoreError>> + Send;
}

#[derive(Clone)]
pub struct MySqlUserStore {
    pub db: MySqlPool,
}

impl MySqlUserStore {
    pub fn new(db: Pool<sqlx::MySql>) -> Self {
        Self { db }
    }
}

impl UserStore for MySqlUserStore {
    async fn register(
        &self,
        tg_user_id: i64,
        chat_id: i64,
    ) -> Result<RegisterOutcome, UserStoreError> {
        let mut tx = self.db.begin().await?;

        let insert_result = sqlx::query(
            r#"
        INSERT INTO users (
            tg_user_id,
            chat_id,
            is_active
        )
        VALUES (?, ?, TRUE)
        "#,
        )
        .bind(tg_user_id)
        .bind(chat_id)
        .execute(&mut *tx)
        .await;

        match insert_result {
            Ok(_) => {
                tx.commit().await?;
                Ok(RegisterOutcome::Created)
            }

            Err(sqlx::Error::Database(ref err)) if err.is_unique_violation() => {
                sqlx::query(
                    r#"
                UPDATE users
                SET
                    chat_id = ?,
                    is_active = TRUE
                WHERE tg_user_id = ?
                "#,
                )
                .bind(chat_id)
                .bind(tg_user_id)
                .execute(&mut *tx)
                .await?;

                tx.commit().await?;

                Ok(RegisterOutcome::Existing)
            }

            Err(err) => Err(UserStoreError::Database(err)),
        }
    }

    async fn get(&self, tg_user_id: i64) -> Result<YalomUser, UserStoreError> {
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
        .bind(tg_user_id)
        .fetch_optional(&self.db)
        .await
        .map_err(UserStoreError::Database)?
        .ok_or(UserStoreError::NotFound)?;

        Ok(user)
    }

    async fn update_bot_settings(
        &self,
        tg_user_id: i64,
        active: bool,
    ) -> Result<(), UserStoreError> {
        let result = sqlx::query(
            r#"
            UPDATE users SET is_active = ?
            WHERE tg_user_id = ?
        "#,
        )
        .bind(active)
        .bind(tg_user_id)
        .execute(&self.db)
        .await
        .map_err(UserStoreError::Database)?;

        if result.rows_affected() == 0 {
            return Err(UserStoreError::NotFound);
        }

        Ok(())
    }
}
