use sqlx::Result;
use sqlx::SqlitePool;

pub(crate) static SQLX_POOL: tokio::sync::OnceCell<sqlx::SqlitePool> =
    tokio::sync::OnceCell::const_new();

/// Get the MongoDB client instance
pub async fn get_sqlite_pool() -> Result<&'static SqlitePool> {
    SQLX_POOL
        .get_or_try_init(|| async {
            let sqlite_pool = SqlitePool::connect("sqlite:eventbus.db?mode=rwc").await?;

            Ok(sqlite_pool)
        })
        .await
}

pub async fn init() -> Result<()> {
    let pool = get_sqlite_pool().await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS events (
            id TEXT PRIMARY KEY,
            message TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS can_messages (
            id INTEGER NOT NULL,
            dlc INTEGER NOT NULL,
            data TEXT NOT NULL,
            speed INTEGER NOT NULL,
            temperature INTEGER NOT NULL,
            pressure INTEGER NOT NULL,
            timestamp TEXT NOT NULL,
            PRIMARY KEY (id, timestamp)
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}
