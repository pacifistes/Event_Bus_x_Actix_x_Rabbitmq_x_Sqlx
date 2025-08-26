use sqlx::Result;
use sqlx::SqlitePool;

pub(crate) static SQLX_POOL: tokio::sync::OnceCell<sqlx::SqlitePool> =
    tokio::sync::OnceCell::const_new();

/// Get the SQLite pool instance
pub async fn get_pool() -> Result<&'static SqlitePool> {
    SQLX_POOL
        .get_or_try_init(|| async {
            let sqlite_pool = SqlitePool::connect("sqlite:eventbus.db?mode=rwc").await?;

            Ok(sqlite_pool)
        })
        .await
}

pub async fn init() -> Result<()> {
    let pool = get_pool().await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS can_messages (
            id INTEGER NOT NULL,
            dlc INTEGER NOT NULL,
            data TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            PRIMARY KEY (id, timestamp)
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}
