pub mod controller;
pub mod model;
pub mod service;

use actix_web::{get, web, HttpResponse, Result};
use serde_json;

use crate::common::error::AppError;

pub use model::DrivingStep;

#[get("/driving-steps")]
pub async fn list() -> Result<HttpResponse, AppError> {
    let steps = controller::list().await?;
    Ok(HttpResponse::Ok().json(steps))
}

#[get("/driving-steps/last")]
pub async fn get_last() -> Result<HttpResponse, AppError> {
    let step = controller::get_last().await?;
    match step {
        Some(step) => Ok(HttpResponse::Ok().json(step)),
        None => {
            Ok(HttpResponse::NotFound()
                .json(serde_json::json!({"error": "No driving steps found"})))
        }
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list).service(get_last);
}
