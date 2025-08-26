use lapin::Channel;
use tokio::sync::broadcast;

use super::model::NewEvent;
use super::service;
use crate::common::error::AppError;
use crate::core::websocket::BusMessage;
use crate::features::event::model::Event;

pub(crate) async fn list() -> Result<Vec<Event>, AppError> {
    service::list().await
}

pub(crate) async fn create(
    new_event: NewEvent,
    tx: &broadcast::Sender<BusMessage>,
    channel: &Channel,
) -> Result<Event, AppError> {
    // Create event in database
    let event = service::create(new_event).await?;

    // Publish to RabbitMQ
    if let Err(e) = crate::config::rabbitmq::publish_event(channel, &event, "events").await {
        eprintln!("RabbitMQ publish error: {e:?}");
        return Err(AppError::internal_server_error(e.to_string()));
    }

    // Broadcast to WebSocket connections
    let _ = tx.send(BusMessage::Event(event.clone()));

    Ok(event)
}
