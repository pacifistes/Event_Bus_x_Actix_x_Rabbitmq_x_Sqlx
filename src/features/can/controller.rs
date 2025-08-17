use lapin::Channel;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::{
    common::error::AppError,
    core::websocket::BusMessage,
    features::{can::model::NewCanMessage, event::model::Event},
};

use super::model::CanMessage;

pub(crate) async fn list() -> Result<Vec<CanMessage>, AppError> {
    let pool = crate::config::sqlite::get_sqlite_pool().await?;

    let rows = sqlx::query("SELECT id, dlc, data, speed, temperature, pressure, timestamp FROM can_messages ORDER BY timestamp DESC")
        .fetch_all(pool)
        .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            use sqlx::Row;
            let data_str: String = row.get("data");
            let data: [u8; 8] = serde_json::from_str(&data_str).unwrap_or([0; 8]);

            CanMessage {
                id: row.get::<i64, _>("id") as u16,
                dlc: row.get::<i64, _>("dlc") as u8,
                data,
                speed: row.get::<i64, _>("speed") as u8,
                temperature: row.get::<i64, _>("temperature") as u8,
                pressure: row.get::<i64, _>("pressure") as u16,
                timestamp: row.get("timestamp"),
            }
        })
        .collect())
}

pub(crate) async fn create(
    new_can: NewCanMessage,
    tx: &broadcast::Sender<BusMessage>,
    channel: &Channel,
) -> Result<CanMessage, AppError> {
    let pool = crate::config::sqlite::get_sqlite_pool().await?;

    let can_msg = CanMessage::new(
        new_can.id,
        new_can.speed,
        new_can.temperature,
        new_can.pressure,
    );

    // DB - Store CAN message
    if let Err(e) = sqlx::query(
        "INSERT INTO can_messages (id, dlc, data, speed, temperature, pressure, timestamp) VALUES ($1, $2, $3, $4, $5, $6, $7)"
    )
    .bind(can_msg.id as i64)
    .bind(can_msg.dlc as i64)
    .bind(serde_json::to_string(&can_msg.data).unwrap())
    .bind(can_msg.speed as i64)
    .bind(can_msg.temperature as i64)
    .bind(can_msg.pressure as i64)
    .bind(&can_msg.timestamp)
    .execute(pool)
    .await
    {
        eprintln!("DB error: {e}");
        return Err(AppError::internal_server_error(e.to_string()));
    }

    // Send via RabbitMQ
    if let Err(e) = crate::config::rabbitmq::publish_event(
        &channel,
        &Event {
            id: Uuid::new_v4(),
            message: format!(
                "CAN message: ID={:#X}, speed={}, temp={}, pressure={}",
                can_msg.id, can_msg.speed, can_msg.temperature, can_msg.pressure
            ),
        },
        "events",
    )
    .await
    {
        return Err(AppError::internal_server_error(e.to_string()));
    }

    // Or send directly inside the broadcast
    let _ = tx.send(BusMessage::Can(can_msg.clone()));

    Ok(can_msg)
}
