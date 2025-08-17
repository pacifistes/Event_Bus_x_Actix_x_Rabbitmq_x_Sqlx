use actix_web::web::Data;
use actix_web::{get, web, Error, HttpResponse, Responder};
use actix_web_lab::sse;
use tokio::sync::broadcast;

use crate::BusMessage;

/* ---------- SSE with actix-web-lab (GET /stream-lab) ---------- */
#[get("/stream-lab")]
async fn stream_lab_events(tx: Data<broadcast::Sender<BusMessage>>) -> impl Responder {
    let mut rx = tx.subscribe();

    let stream = async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    let data = serde_json::to_string(&msg).unwrap_or_else(|_| "{}".to_string());
                    yield Ok::<_, Error>(sse::Event::Data(sse::Data::new(data)));
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => break,
            }
        }
    };

    sse::Sse::from_stream(stream)
}

/* ---------- SSE (GET /stream) ---------- */
#[get("/stream")]
async fn stream_events(tx: Data<broadcast::Sender<BusMessage>>) -> impl Responder {
    let mut rx = tx.subscribe();

    let stream = async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    let line = format!("data: {}\n\n", serde_json::to_string(&msg).unwrap());
                    yield Ok::<_, Error>(actix_web::web::Bytes::from(line));
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => break,
            }
        }
    };

    HttpResponse::Ok()
        .insert_header(("Content-Type", "text/event-stream"))
        .insert_header(("Cache-Control", "no-cache"))
        .insert_header(("X-Accel-Buffering", "no"))
        .streaming(stream)
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(stream_events);
    cfg.service(stream_lab_events);
}
