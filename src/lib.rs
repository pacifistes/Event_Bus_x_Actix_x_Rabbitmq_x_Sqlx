/// Library exports for the CAN Bus + RabbitMQ + SQLx real-time system
/// This allows examples to import the actual structs and modules
pub mod common;
pub mod config;
pub mod core;
pub mod features;

// Re-export commonly used items for convenience
pub use core::can::CanMessage;
pub use features::driving_step::DrivingStep;
