use futures_util::StreamExt;
use lapin::{options::*, types::FieldTable, Channel, Connection, ConnectionProperties};
use sqlx::Row;
use tokio::sync::broadcast;
use lapin::Result;
use serde_json;

use crate::core::can::CanMessage;
use crate::features::driving_step::DrivingStep;

pub const QUEUE_NAME: &str = "step_names";
pub const CONSUMER_TAG: &str = "step-name-broadcaster";

pub async fn connect() -> Result<Connection> {
    Connection::connect(
        "amqp://guest:guest@127.0.0.1:5672/%2f",
        ConnectionProperties::default(),
    )
    .await
}

pub async fn create_step_name_channel(connection: &Connection) -> Result<Channel> {
    let channel = connection.create_channel().await?;
    channel
        .queue_declare(
            QUEUE_NAME,
            QueueDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;

    Ok(channel)
}

pub async fn consume_step_names(
    channel: &Channel,
    tx: &broadcast::Sender<DrivingStep>,
) -> Result<()> {
    let mut consumer = channel
        .basic_consume(
            QUEUE_NAME,
            CONSUMER_TAG,
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    let tx_clone = tx.clone();
    tokio::spawn(async move {
        while let Some(delivery) = consumer.next().await {
            if let Ok(delivery) = delivery {
                // Try to parse as new format with endianness
                if let Ok(step_data) = serde_json::from_slice::<serde_json::Value>(&delivery.data) {
                    let (step_name, endian) = if let (Some(name), Some(endian_val)) = 
                        (step_data.get("step_name"), step_data.get("endian")) {
                        // New format: {"step_name": "...", "endian": "..."}
                        if let (Some(name_str), Some(endian_str)) = 
                            (name.as_str(), endian_val.as_str()) {
                            (name_str.to_string(), endian_str.to_string())
                        } else {
                            continue; // Skip malformed messages
                        }
                    } else {
                        continue; // Skip malformed messages
                    };

                    println!("📨 RabbitMQ received step_name: '{}', endian: '{}'", step_name, endian);
                    
                    // Set environment variable for this reconstruction
                    std::env::set_var("ENDIAN", &endian);
                    
                    // Reconstruct DrivingStep from database using step_name
                    if let Ok(pool) = crate::config::sqlite::get_pool().await {
                        // Get the latest 7 CAN messages for the specified endianness
                        if let Ok(rows) = sqlx::query(
                            "SELECT id, dlc, data, timestamp FROM can_messages WHERE endian = ? ORDER BY timestamp DESC LIMIT 7"
                        )
                        .bind(&endian)
                        .fetch_all(pool)
                        .await {
                            let mut retrieved_can_messages = Vec::new();
                            for row in rows {
                                if let (Ok(id), Ok(dlc), Ok(data_json), Ok(timestamp)) = (
                                    row.try_get::<i64, _>("id"),
                                    row.try_get::<i64, _>("dlc"), 
                                    row.try_get::<String, _>("data"),
                                    row.try_get::<String, _>("timestamp")
                                ) {
                                    if let Ok(data) = serde_json::from_str::<[u8; 8]>(&data_json) {
                                        retrieved_can_messages.push(CanMessage {
                                            id: id as u16,
                                            dlc: dlc as u8,
                                            data,
                                            timestamp,
                                        });
                                    }
                                }
                            }

                            // Try to reconstruct DrivingStep if we have enough messages
                            if retrieved_can_messages.len() >= 7 {
                                match crate::features::driving_step::model::DrivingStep::from_can_messages(
                                    &retrieved_can_messages, 
                                    step_name.clone()
                                ) {
                                    Ok(reconstructed_step) => {
                                        println!("🔄 RabbitMQ Stream: Successfully reconstructed DrivingStep '{}'", reconstructed_step.step_name);
                                        // Send reconstructed DrivingStep to WebSocket clients
                                        let _ = tx_clone.send(reconstructed_step);
                                    }
                                    Err(e) => {
                                        println!("❌ RabbitMQ Stream: Failed to reconstruct DrivingStep: {}", e);
                                    }
                                }
                            } else {
                                println!("❌ RabbitMQ Stream: Not enough CAN messages ({}) to reconstruct DrivingStep", retrieved_can_messages.len());
                            }
                        }
                    }
                }
                let _ = delivery.ack(BasicAckOptions::default()).await;
            }
        }
    });

    Ok(())
}