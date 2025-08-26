use crate::common::error::AppError;
use crate::features::driving_step::model::DrivingStep;
use crate::features::driving_step::service;

pub async fn list() -> Result<Vec<DrivingStep>, AppError> {
    service::get_all_steps().await
}

pub async fn get_last() -> Result<Option<DrivingStep>, AppError> {
    service::get_last_step().await
}
