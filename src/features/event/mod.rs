mod controller;
pub mod model;
mod service;

use actix_web::web::Data;
use actix_web::{get, post, web, HttpResponse};
use lapin::Channel;
use tokio::sync::broadcast;

use model::NewEvent;

use crate::common::error::AppError;
use crate::core::websocket::BusMessage;

#[post("/events")]
async fn create_event(
    channel: Data<Channel>,
    tx: Data<broadcast::Sender<BusMessage>>,
    payload: web::Json<NewEvent>,
) -> Result<HttpResponse, AppError> {
    let event = controller::create(payload.into_inner(), &tx, &channel).await?;

    Ok(HttpResponse::Ok().json(&event))
}

#[get("/events")]
async fn list_events() -> Result<HttpResponse, AppError> {
    let rows = controller::list().await?;
    Ok(HttpResponse::Ok().json(rows))
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(create_event).service(list_events);
}
