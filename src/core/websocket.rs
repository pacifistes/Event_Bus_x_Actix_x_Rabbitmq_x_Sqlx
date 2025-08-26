use actix::AsyncContext;
use actix::{Actor, StreamHandler};
use actix_web::web::Data;
use actix_web::{get, web, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use lapin::Channel;

use sqlx::SqlitePool;
use tokio::sync::broadcast;

use crate::common::error::AppError;
use crate::features::driving_step::DrivingStep;

#[derive(actix::Message)]
#[rtype(result = "()")]
struct BroadcastMessage(String);

struct WsConn {
    rx: broadcast::Receiver<DrivingStep>,
    pool: SqlitePool,
    channel: Channel,
}

impl Actor for WsConn {
    type Context = ws::WebsocketContext<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        let mut rx = self.rx.resubscribe();
        let addr = ctx.address();

        tokio::spawn(async move {
            while let Ok(driving_step) = rx.recv().await {
                // Handle DrivingStep messages for display
                println!("\n🚗 DRIVING STEP RECEIVED VIA WEBSOCKET:");
                driving_step.print_status();
                driving_step.show_can_messages();

                if let Ok(txt) = serde_json::to_string(&driving_step) {
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
            println!("🔍 Received message: {}", &text);
            // Try parsing as DrivingStep
            if let Ok(driving_step) = serde_json::from_str::<DrivingStep>(&text) {
                let pool = self.pool.clone();
                let channel = self.channel.clone();
                let step_name = driving_step.step_name.clone();

                tokio::spawn(async move {
                    // Convert to CAN messages and store
                    let can_messages = driving_step.to_can_messages();

                    // Store each CAN message in database
                    for can_msg in &can_messages {
                        match sqlx::query(
                            "INSERT INTO can_messages (id, dlc, data, timestamp) 
                             VALUES (?, ?, ?, ?)",
                        )
                        .bind(can_msg.id as i64)
                        .bind(can_msg.dlc as i64)
                        .bind(serde_json::to_string(&can_msg.data).unwrap_or_default())
                        .bind(&can_msg.timestamp)
                        .execute(&pool)
                        .await
                        {
                            Ok(_) => println!("✅ Stored CAN message ID: 0x{:03X}", can_msg.id),
                            Err(e) => println!(
                                "❌ Failed to store CAN message ID: 0x{:03X}, Error: {}",
                                can_msg.id, e
                            ),
                        }
                    }

                    // Send step_name to RabbitMQ
                    if let Ok(payload) = serde_json::to_vec(&step_name) {
                        let _ = channel
                            .basic_publish(
                                "",                                  // Use default exchange for direct queue publishing
                                crate::config::rabbitmq::QUEUE_NAME, // Direct to queue name
                                lapin::options::BasicPublishOptions::default(),
                                &payload,
                                lapin::BasicProperties::default(),
                            )
                            .await;
                    }

                    println!(
                        "📡 Processed DrivingStep '{}' via WebSocket: {} CAN messages stored, step_name sent to RabbitMQ",
                        step_name,
                        can_messages.len()
                    );
                });
            } else {
                ctx.text(r#"{"error":"Invalid format, expected DrivingStep JSON"}"#);
            }
        }
    }
}

#[get("/ws")]
async fn ws_handler(
    req: HttpRequest,
    stream: web::Payload,
    channel: Data<Channel>,
    tx: Data<broadcast::Sender<DrivingStep>>,
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
