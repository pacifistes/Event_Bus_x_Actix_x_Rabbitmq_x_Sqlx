use serde_json;
use sqlx::Row;
use std::collections::HashMap;

use crate::common::error::AppError;
use crate::core::can::CanMessage;
use crate::features::driving_step::model::DrivingStep;

pub async fn get_all_steps() -> Result<Vec<DrivingStep>, AppError> {
    let pool = crate::config::sqlite::get_pool().await?;

    // Get all CAN messages ordered by timestamp
    let rows = sqlx::query(
        "SELECT id, dlc, data, timestamp 
         FROM can_messages ORDER BY timestamp ASC",
    )
    .fetch_all(pool)
    .await?;

    let mut can_messages = Vec::new();
    for row in rows {
        let id: i64 = row.try_get("id")?;
        let dlc: i64 = row.try_get("dlc")?;
        let data_json: String = row.try_get("data")?;
        let timestamp: String = row.try_get("timestamp")?;

        let data: [u8; 8] = serde_json::from_str(&data_json)?;

        can_messages.push(CanMessage {
            id: id as u16,
            dlc: dlc as u8,
            data,
            timestamp,
        });
    }

    // Group CAN messages by timestamp to reconstruct driving steps
    let mut grouped_messages: HashMap<String, Vec<CanMessage>> = HashMap::new();

    for msg in can_messages {
        grouped_messages
            .entry(msg.timestamp.clone())
            .or_insert_with(Vec::new)
            .push(msg);
    }

    let mut steps = Vec::new();
    let mut step_counter = 1;

    for (timestamp, messages) in grouped_messages {
        if messages.len() >= 7 {
            // We need 7 CAN messages for a complete DrivingStep
            let step_name = format!("Step_{}", step_counter);
            match DrivingStep::from_can_messages(&messages, step_name) {
                Ok(step) => {
                    steps.push(step);
                    step_counter += 1;
                }
                Err(e) => {
                    println!(
                        "⚠️ Could not reconstruct driving step from timestamp {}: {}",
                        timestamp, e
                    );
                }
            }
        }
    }

    Ok(steps)
}

pub async fn get_last_step() -> Result<Option<DrivingStep>, AppError> {
    let pool = crate::config::sqlite::get_pool().await?;

    // Get the latest 7 CAN messages (should contain one complete DrivingStep)
    let rows = sqlx::query(
        "SELECT id, dlc, data, timestamp 
         FROM can_messages ORDER BY timestamp DESC LIMIT 7",
    )
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok(None);
    }

    let mut can_messages = Vec::new();
    for row in rows {
        let id: i64 = row.try_get("id")?;
        let dlc: i64 = row.try_get("dlc")?;
        let data_json: String = row.try_get("data")?;
        let timestamp: String = row.try_get("timestamp")?;

        let data: [u8; 8] = serde_json::from_str(&data_json)?;

        can_messages.push(CanMessage {
            id: id as u16,
            dlc: dlc as u8,
            data,
            timestamp,
        });
    }

    // Try to reconstruct DrivingStep from the latest CAN messages
    if can_messages.len() >= 7 {
        let step_name = "Latest_Step".to_string();
        match DrivingStep::from_can_messages(&can_messages, step_name) {
            Ok(step) => Ok(Some(step)),
            Err(e) => {
                println!("⚠️ Could not reconstruct latest driving step: {}", e);
                Ok(None)
            }
        }
    } else {
        println!(
            "⚠️ Not enough CAN messages ({}) to reconstruct driving step",
            can_messages.len()
        );
        Ok(None)
    }
}
