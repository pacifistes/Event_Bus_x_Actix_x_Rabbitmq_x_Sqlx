use lapin::Channel;
use tokio::sync::broadcast;
use uuid::Uuid;

use super::model::{Event, NewEvent};

use crate::common::error::AppError;
use crate::core::websocket::BusMessage;

pub(crate) async fn list() -> Result<Vec<Event>, AppError> {
    let pool = crate::config::sqlite::get_sqlite_pool().await?;
    sqlx::query_as::<_, Event>("SELECT id, message FROM events ORDER BY id DESC")
        .fetch_all(pool)
        .await
        .map_err(AppError::from)
}

pub(crate) async fn create(
    new_event: NewEvent,
    tx: &broadcast::Sender<BusMessage>,
    channel: &Channel,
) -> Result<Event, AppError> {
    let event = Event {
        id: Uuid::new_v4(),
        message: new_event.message.clone(),
    };

    let pool = crate::config::sqlite::get_sqlite_pool().await?;
    if let Err(e) = sqlx::query("INSERT INTO events (id, message) VALUES ($1, $2)")
        .bind(event.id.to_string())
        .bind(&event.message)
        .execute(pool)
        .await
    {
        eprintln!("DB error: {e}");
        return Err(AppError::internal_server_error(e.to_string()));
    }

    // Send via RabbitMQ
    if let Err(e) = crate::config::rabbitmq::publish_event(&channel, &event, "events").await {
        eprintln!("RabbitMQ publish error: {e:?}");
        return Err(AppError::internal_server_error(e.to_string()));
    }

    // Or send directly inside the broadcast
    let _ = tx.send(BusMessage::Event(event.clone()));

    Ok(event)
}
