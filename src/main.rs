mod common;
mod config;
mod core;
mod features;

use actix_web::middleware;
use actix_web::{web::Data, App, HttpServer};
use tokio::sync::broadcast;

use crate::features::driving_step::DrivingStep;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_BACKTRACE", "1");
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "actix_web=debug,info,warn");
    }
    env_logger::init();

    let (tx, _rx) = broadcast::channel::<DrivingStep>(512);

    // RabbitMQ
    let rabit_connection = config::rabbitmq::connect()
        .await
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error.to_string()))?;
    let channel = config::rabbitmq::create_step_name_channel(&rabit_connection)
        .await
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error.to_string()))?;
    config::rabbitmq::consume_step_names(&channel, &tx)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    // SQLite
    config::sqlite::init()
        .await
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error.to_string()))?;

    // Server HTTP
    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::new(
                "%{r}a %r %s %b %{Referer}i %{User-Agent}i %T",
            ))
            .app_data(Data::new(channel.clone()))
            .app_data(Data::new(tx.clone()))
            .configure(features::driving_step::configure)
            .configure(core::stream::configure)
            .configure(core::websocket::configure)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await?;

    Ok(())
}
