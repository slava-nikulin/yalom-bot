use sqlx::mysql::MySqlPoolOptions;
use yalom_bot::config::migrate_config::MigrationConfig;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let cfg = MigrationConfig::load()?;

    let db = MySqlPoolOptions::new()
        .max_connections(1)
        .connect(&cfg.database.url)
        .await?;

    sqlx::migrate!("./migrations").run(&db).await?;

    db.close().await;
    Ok(())
}
