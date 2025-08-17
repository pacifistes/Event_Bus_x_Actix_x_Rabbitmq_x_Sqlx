use futures_util::StreamExt;
use lapin::{options::*, types::FieldTable, Channel, Connection, ConnectionProperties};
use tokio::sync::broadcast;

use crate::features::event::model::Event;
use crate::BusMessage;
use lapin::{BasicProperties, Result};

pub const QUEUE_NAME: &str = "events";
pub const CONSUMER_TAG: &str = "realtime-broadcaster";

pub async fn connect() -> Result<Connection> {
    Connection::connect(
        "amqp://guest:guest@127.0.0.1:5672/%2f",
        ConnectionProperties::default(),
    )
    .await
}

pub async fn create_event_channel(connection: &Connection) -> Result<Channel> {
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

pub async fn consume_events(channel: &Channel, tx: &broadcast::Sender<BusMessage>) -> Result<()> {
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
                if let Ok(evt) = serde_json::from_slice::<Event>(&delivery.data) {
                    let _ = tx_clone.send(BusMessage::Event(evt));
                }
                let _ = delivery.ack(BasicAckOptions::default()).await;
            }
        }
    });

    Ok(())
}

/* ---------- RabbitMQ publish ---------- */
pub async fn publish_event<T: serde::Serialize>(
    channel: &Channel,
    event: &T,
    routing_key: &str,
) -> anyhow::Result<()> {
    let payload = serde_json::to_vec(event)?;
    channel
        .basic_publish(
            "",          // exchange par défaut
            routing_key, // queue
            BasicPublishOptions::default(),
            &payload,
            BasicProperties::default(),
        )
        .await?
        .await?; // wait for broker confirm
    Ok(())
}
