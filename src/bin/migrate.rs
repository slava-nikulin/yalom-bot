use sqlx::mysql::MySqlPoolOptions;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let db = MySqlPoolOptions::new()
        .max_connections(1)
        .connect("")
        .await?;

    sqlx::migrate!("./migrations").run(&db).await?;

    db.close().await;

    Ok(())
}
