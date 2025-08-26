use uuid::Uuid;

use super::model::{Event, NewEvent};
use crate::common::error::AppError;

/// Get all events from database
pub async fn list() -> Result<Vec<Event>, AppError> {
    let pool = crate::config::sqlite::get_pool().await?;

    sqlx::query_as::<_, Event>("SELECT id, message FROM events ORDER BY id DESC")
        .fetch_all(pool)
        .await
        .map_err(AppError::from)
}

/// Create a new event (database only)
pub async fn create(new_event: NewEvent) -> Result<Event, AppError> {
    let pool = crate::config::sqlite::get_pool().await?;

    let event = Event {
        id: Uuid::new_v4(),
        message: new_event.message.clone(),
    };

    // Store in database
    sqlx::query("INSERT INTO events (id, message) VALUES ($1, $2)")
        .bind(event.id.to_string())
        .bind(&event.message)
        .execute(pool)
        .await
        .map_err(AppError::from)?;

    Ok(event)
}
