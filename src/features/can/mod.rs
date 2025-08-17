mod controller;
pub mod model;

use actix_web::web::Data;
use actix_web::{get, post, web, HttpResponse};
use lapin::Channel;
use tokio::sync::broadcast;

use crate::common::error::AppError;
use crate::features::can::model::NewCanMessage;
use crate::BusMessage;

#[post("/can")]
async fn create_can_message(
    channel: Data<Channel>,
    tx: Data<broadcast::Sender<BusMessage>>,
    payload: web::Json<NewCanMessage>,
) -> Result<HttpResponse, AppError> {
    let can_msg = controller::create(payload.into_inner(), &tx, &channel).await?;

    Ok(HttpResponse::Ok().json(&can_msg))
}

#[get("/can")]
async fn list_can_messages() -> Result<HttpResponse, AppError> {
    let can_messages = controller::list().await?;

    Ok(HttpResponse::Ok().json(can_messages))
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(create_can_message).service(list_can_messages);
}
