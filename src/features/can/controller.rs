use lapin::Channel;
use tokio::sync::broadcast;
use uuid::Uuid;

use super::model::{CanMessage, NewCanMessage};
use super::service;
use crate::common::error::AppError;
use crate::core::websocket::BusMessage;
use crate::features::event::model::Event;

pub(crate) async fn list() -> Result<Vec<CanMessage>, AppError> {
    service::list().await
}

pub(crate) async fn create(
    new_can: NewCanMessage,
    tx: &broadcast::Sender<BusMessage>,
    channel: &Channel,
) -> Result<CanMessage, AppError> {
    let can_msg = service::create(new_can).await?;

    let event = Event {
        id: Uuid::new_v4(),
        message: format!(
            "CAN message: ID={:#X}, speed={}, temp={}, pressure={}",
            can_msg.id, can_msg.speed, can_msg.temperature, can_msg.pressure
        ),
    };

    if let Err(e) = crate::config::rabbitmq::publish_event(channel, &event, "events").await {
        return Err(AppError::internal_server_error(e.to_string()));
    }

    // Broadcast to WebSocket connections
    let _ = tx.send(BusMessage::Can(can_msg.clone()));

    Ok(can_msg)
}
