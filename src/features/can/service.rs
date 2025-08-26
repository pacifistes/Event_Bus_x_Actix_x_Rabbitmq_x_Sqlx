use sqlx::Row;

use super::model::{CanMessage, NewCanMessage};
use crate::common::error::AppError;

/// Get all CAN messages from database
pub async fn list() -> Result<Vec<CanMessage>, AppError> {
    let pool = crate::config::sqlite::get_pool().await?;

    let rows = sqlx::query(
        "SELECT id, dlc, data, speed, temperature, pressure, timestamp FROM can_messages ORDER BY timestamp DESC"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
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

/// Create a new CAN message (database only)
pub async fn create(new_can: NewCanMessage) -> Result<CanMessage, AppError> {
    let pool = crate::config::sqlite::get_pool().await?;

    // Create the CAN message from input
    let can_msg = CanMessage::new(
        new_can.id,
        new_can.speed,
        new_can.temperature,
        new_can.pressure,
    );

    // Store in database
    sqlx::query(
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
    .map_err(AppError::from)?;

    Ok(can_msg)
}
