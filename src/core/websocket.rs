use actix::AsyncContext;
use actix::{Actor, StreamHandler};
use actix_web::web::Data;
use actix_web::{get, web, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use lapin::Channel;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::common::error::AppError;
use crate::features::can::model::{CanMessage, NewCanMessage};
use crate::features::event::model::{Event, NewEvent};

#[derive(actix::Message)]
#[rtype(result = "()")]
struct BroadcastMessage(String);

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum BusMessage {
    Event(Event),
    Can(CanMessage),
}

struct WsConn {
    rx: broadcast::Receiver<BusMessage>,
    pool: SqlitePool,
    channel: Channel,
}

impl Actor for WsConn {
    type Context = ws::WebsocketContext<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        let mut rx = self.rx.resubscribe();
        let addr = ctx.address();

        tokio::spawn(async move {
            while let Ok(msg) = rx.recv().await {
                if let Ok(txt) = serde_json::to_string(&msg) {
                    addr.do_send(BroadcastMessage(txt));
                }
            }
        });
    }
}

impl actix::Handler<BroadcastMessage> for WsConn {
    type Result = ();

    fn handle(&mut self, msg: BroadcastMessage, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsConn {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        if let Ok(ws::Message::Text(text)) = msg {
            // Try parsing as regular Event first
            if let Ok(new_evt) = serde_json::from_str::<NewEvent>(&text) {
                let pool = self.pool.clone();
                let channel = self.channel.clone();
                tokio::spawn(async move {
                    let evt = Event {
                        id: Uuid::new_v4(),
                        message: new_evt.message,
                    };
                    let _ = sqlx::query("INSERT INTO events (id, message) VALUES ($1, $2)")
                        .bind(evt.id.to_string())
                        .bind(&evt.message)
                        .execute(&pool)
                        .await;
                    let _ = crate::config::rabbitmq::publish_event(&channel, &evt, "events").await;
                });
            }
            // Try parsing as CAN message
            else if let Ok(new_can) = serde_json::from_str::<NewCanMessage>(&text) {
                let pool = self.pool.clone();
                let channel = self.channel.clone();
                tokio::spawn(async move {
                    let can_msg = CanMessage::new(
                        new_can.id,
                        new_can.speed,
                        new_can.temperature,
                        new_can.pressure,
                    );

                    let _ = sqlx::query(
                        "INSERT INTO can_messages (id, dlc, data, speed, temperature, pressure, timestamp) VALUES ($1, $2, $3, $4, $5, $6, $7)"
                    )
                    .bind(can_msg.id as i64)
                    .bind(can_msg.dlc as i64)
                    .bind(serde_json::to_string(&can_msg.data).unwrap())
                    .bind(can_msg.speed as i64)
                    .bind(can_msg.temperature as i64)
                    .bind(can_msg.pressure as i64)
                    .bind(&can_msg.timestamp)
                    .execute(&pool)
                    .await;

                    let _ = crate::config::rabbitmq::publish_event(
                        &channel,
                        &Event {
                            id: Uuid::new_v4(),
                            message: format!(
                                "CAN via WS: ID={:#X}, speed={}, temp={}, pressure={}",
                                can_msg.id, can_msg.speed, can_msg.temperature, can_msg.pressure
                            ),
                        },
                        "events",
                    )
                    .await;
                });
            } else {
                ctx.text(r#"{"error":"Invalid format, expected Event {\"message\": \"...\"} or CAN {\"id\": 256, \"speed\": 120, \"temperature\": 90, \"pressure\": 512}"}"#);
            }
        }
    }
}

#[get("/ws")]
async fn ws_handler(
    req: HttpRequest,
    stream: web::Payload,
    channel: Data<Channel>,
    tx: Data<broadcast::Sender<BusMessage>>,
) -> Result<HttpResponse, AppError> {
    let rx = tx.subscribe();
    let pool = crate::config::sqlite::get_pool().await?;
    let actor = WsConn {
        rx,
        pool: pool.to_owned(),
        channel: channel.get_ref().clone(),
    };
    ws::start(actor, &req, stream).map_err(AppError::from)
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(ws_handler);
}
