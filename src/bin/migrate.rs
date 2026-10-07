use sqlx::mysql::MySqlPoolOptions;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let database_url = std::env::var("DATABASE_URL")?;

    let db = MySqlPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&db).await?;

    db.close().await;

    Ok(())
}
