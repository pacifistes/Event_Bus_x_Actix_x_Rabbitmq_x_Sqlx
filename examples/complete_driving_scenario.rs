use canbus_rmq_realtime::config::rabbitmq::QUEUE_NAME;
/// Complete driving scenario that uses the actual structs from the features folder
/// This example demonstrates the complete flow with all 6 scenario steps:
/// 1. Create DrivingStep scenarios (Vehicle Start, First Gear, Acceleration, Highway Cruise, Emergency Braking, Vehicle Stop)
/// 2. Convert each to CAN messages
/// 3. Store in SQLx database
/// 4. Send to RabbitMQ
/// 5. Simulate WebSocket/Stream retrieving and reconstructing
use lapin::options::BasicPublishOptions;
use lapin::{BasicProperties, Channel, Connection, ConnectionProperties};
use serde_json;
use sqlx::SqlitePool;
use tokio;
use tokio_stream::StreamExt;

// Import the actual structs from the main crate library
use canbus_rmq_realtime::features::driving_step::model::{
    ClimateData, EngineData, VehicleSpeedData,
};
use canbus_rmq_realtime::{CanMessage, DrivingStep};

/// Store CAN messages in SQLite database
async fn store_can_messages(
    pool: &SqlitePool,
    can_messages: &[CanMessage],
) -> Result<(), Box<dyn std::error::Error>> {
    for can_msg in can_messages {
        sqlx::query(
            "INSERT INTO can_messages (id, dlc, data, timestamp, endian) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(can_msg.id as i64)
        .bind(can_msg.dlc as i64)
        .bind(serde_json::to_string(&can_msg.data)?)
        .bind(&can_msg.timestamp)
        .bind(std::env::var("ENDIAN").unwrap_or_else(|_| "little".to_string()))
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// Send step_name and endianness to RabbitMQ
async fn send_step_data_to_rabbitmq(
    channel: &Channel,
    step_name: &str,
    endian: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let step_data = serde_json::json!({
        "step_name": step_name,
        "endian": endian
    });
    let payload = serde_json::to_vec(&step_data)?;

    channel
        .basic_publish(
            "",         // Use default exchange for direct queue publishing
            QUEUE_NAME, // Direct to queue name
            BasicPublishOptions::default(),
            &payload,
            BasicProperties::default(),
        )
        .await?;
    Ok(())
}

/// Connect to the server's /stream-lab endpoint to receive DrivingStep broadcasts
async fn connect_to_stream_endpoint() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🌐 Connecting to server /stream-lab endpoint...");

    // Try to connect to the SSE stream
    let client = reqwest::Client::new();
    let response = client
        .get("http://127.0.0.1:8080/stream-lab")
        .send()
        .await?;

    if response.status().is_success() {
        println!("   ✅ Connected to /stream-lab endpoint");

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        // Process the SSE stream with buffering for larger structs
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(bytes) => {
                    // Convert bytes to string and add to buffer
                    if let Ok(chunk_str) = std::str::from_utf8(&bytes) {
                        buffer.push_str(chunk_str);

                        // Process complete SSE events (ending with \n\n)
                        while let Some(event_end) = buffer.find("\n\n") {
                            let event_data = buffer[..event_end].to_string();
                            buffer.drain(..event_end + 2); // Remove processed event including \n\n

                            // Process each line in the event
                            for line in event_data.lines() {
                                if line.starts_with("data: ") {
                                    let json_data = &line[6..]; // Remove "data: " prefix
                                    match serde_json::from_str::<DrivingStep>(json_data) {
                                        Ok(driving_step) => {
                                            println!("\n📻 RECEIVED DRIVINGSTEP FROM STREAM:");
                                            driving_step.print_status();
                                            driving_step.show_can_messages();
                                        }
                                        Err(e) => {
                                            println!("❌ Failed to parse DrivingStep: {}", e);
                                            println!("   Raw JSON: {}", json_data);
                                        }
                                    }
                                }
                            }
                        }

                        // Prevent buffer from growing too large (protect against memory issues)
                        if buffer.len() > 10_000 {
                            println!(
                                "⚠️ Buffer size exceeded 10KB, clearing to prevent memory issues"
                            );
                            buffer.clear();
                        }
                    }
                }
                Err(e) => {
                    println!("   ❌ Stream error: {}", e);
                    break;
                }
            }
        }
    } else {
        println!(
            "   ❌ Failed to connect to /stream-lab: {}",
            response.status()
        );
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚗🇺🇸 COMPLETE REALISTIC DRIVING SIMULATION 🇺🇸🚗");
    println!("═══════════════════════════════════════════════════════════");
    println!("🎯 DEMONSTRATION: DrivingStep → CAN Messages → SQLx → RabbitMQ → Reconstruction");

    // Initialize database schema first
    canbus_rmq_realtime::config::sqlite::init().await?;
    println!("✅ Connected to SQLite database");
    let pool = canbus_rmq_realtime::config::sqlite::get_pool().await?;

    // Setup RabbitMQ connection (optional)
    let rabbitmq_url =
        std::env::var("RABBITMQ_URL").unwrap_or_else(|_| "amqp://127.0.0.1:5672".to_string());
    let (_connection, channel) =
        match Connection::connect(&rabbitmq_url, ConnectionProperties::default()).await {
            Ok(conn) => {
                let ch = conn.create_channel().await?;
                println!("✅ Connected to RabbitMQ");
                (Some(conn), Some(ch))
            }
            Err(e) => {
                println!(
                    "⚠️ Could not connect to RabbitMQ ({}), continuing without it",
                    e
                );
                (None, None)
            }
        };

    // Start stream endpoint connection in background
    let _stream_handle = tokio::spawn(async {
        if let Err(e) = connect_to_stream_endpoint().await {
            println!("   ⚠️ Could not connect to stream endpoint: {}", e);
        }
    });

    // Create realistic driving scenario with all 6 steps
    let scenario = vec![
        // 1. Vehicle Start
        DrivingStep {
            step_name: "Vehicle Start".to_string(),
            engine: EngineData {
                rpm: 800,
                coolant_temp: 20,
                throttle_pos: 0,
                engine_load: 15,
                intake_temp: 25,
                fuel_pressure: 300,
                engine_running: true,
            },
            speed: VehicleSpeedData {
                vehicle_speed: 0.0,
                gear_position: 0, // Park
                wheel_speeds: [0.0, 0.0, 0.0, 0.0],
                abs_active: false,
                traction_control: true,
                cruise_control: false,
            },
            climate: ClimateData {
                cabin_temp: 18,
                target_temp: 20,
                outside_temp: 15,
                fan_speed: 50,
                ac_compressor: false,
                heater: true,
                defrost: false,
                auto_mode: true,
                air_recirculation: false,
            },
            duration_ms: 2000,
        },
        // 2. First Gear Engagement
        DrivingStep {
            step_name: "First Gear Engagement".to_string(),
            engine: EngineData {
                rpm: 1200,
                coolant_temp: 25,
                throttle_pos: 15,
                engine_load: 25,
                intake_temp: 30,
                fuel_pressure: 320,
                engine_running: true,
            },
            speed: VehicleSpeedData {
                vehicle_speed: 0.0,
                gear_position: 1, // First gear
                wheel_speeds: [0.0, 0.0, 0.0, 0.0],
                abs_active: false,
                traction_control: true,
                cruise_control: false,
            },
            climate: ClimateData {
                cabin_temp: 19,
                target_temp: 20,
                outside_temp: 15,
                fan_speed: 60,
                ac_compressor: false,
                heater: true,
                defrost: false,
                auto_mode: true,
                air_recirculation: false,
            },
            duration_ms: 1500,
        },
        // 3. Acceleration
        DrivingStep {
            step_name: "Acceleration".to_string(),
            engine: EngineData {
                rpm: 2500,
                coolant_temp: 45,
                throttle_pos: 45,
                engine_load: 60,
                intake_temp: 35,
                fuel_pressure: 380,
                engine_running: true,
            },
            speed: VehicleSpeedData {
                vehicle_speed: 25.0,
                gear_position: 2, // Second gear
                wheel_speeds: [25.2, 25.0, 24.8, 25.1],
                abs_active: false,
                traction_control: true,
                cruise_control: false,
            },
            climate: ClimateData {
                cabin_temp: 20,
                target_temp: 20,
                outside_temp: 15,
                fan_speed: 40,
                ac_compressor: false,
                heater: false,
                defrost: false,
                auto_mode: true,
                air_recirculation: false,
            },
            duration_ms: 3000,
        },
        // 4. Highway Cruise
        DrivingStep {
            step_name: "Highway Cruise".to_string(),
            engine: EngineData {
                rpm: 2000,
                coolant_temp: 75,
                throttle_pos: 25,
                engine_load: 35,
                intake_temp: 40,
                fuel_pressure: 350,
                engine_running: true,
            },
            speed: VehicleSpeedData {
                vehicle_speed: 90.0,
                gear_position: 5, // Fifth gear
                wheel_speeds: [90.1, 89.9, 90.0, 90.2],
                abs_active: false,
                traction_control: true,
                cruise_control: true,
            },
            climate: ClimateData {
                cabin_temp: 21,
                target_temp: 21,
                outside_temp: 18,
                fan_speed: 30,
                ac_compressor: true,
                heater: false,
                defrost: false,
                auto_mode: true,
                air_recirculation: true,
            },
            duration_ms: 5000,
        },
        // 5. Emergency Braking
        DrivingStep {
            step_name: "Emergency Braking".to_string(),
            engine: EngineData {
                rpm: 1500,
                coolant_temp: 78,
                throttle_pos: 0,
                engine_load: 10,
                intake_temp: 42,
                fuel_pressure: 300,
                engine_running: true,
            },
            speed: VehicleSpeedData {
                vehicle_speed: 45.0,
                gear_position: 3, // Third gear
                wheel_speeds: [44.5, 45.2, 44.8, 45.1],
                abs_active: true, // ABS engaged!
                traction_control: true,
                cruise_control: false,
            },
            climate: ClimateData {
                cabin_temp: 21,
                target_temp: 21,
                outside_temp: 18,
                fan_speed: 30,
                ac_compressor: true,
                heater: false,
                defrost: false,
                auto_mode: true,
                air_recirculation: true,
            },
            duration_ms: 2000,
        },
        // 6. Vehicle Stop
        DrivingStep {
            step_name: "Vehicle Stop".to_string(),
            engine: EngineData {
                rpm: 800,
                coolant_temp: 80,
                throttle_pos: 0,
                engine_load: 15,
                intake_temp: 45,
                fuel_pressure: 300,
                engine_running: true,
            },
            speed: VehicleSpeedData {
                vehicle_speed: 0.0,
                gear_position: 0, // Park
                wheel_speeds: [0.0, 0.0, 0.0, 0.0],
                abs_active: false,
                traction_control: true,
                cruise_control: false,
            },
            climate: ClimateData {
                cabin_temp: 21,
                target_temp: 21,
                outside_temp: 18,
                fan_speed: 25,
                ac_compressor: false,
                heater: false,
                defrost: false,
                auto_mode: true,
                air_recirculation: true,
            },
            duration_ms: 1000,
        },
    ];

    for endian in ["little", "big"] {
        std::env::set_var("ENDIAN", endian);
        let is_big_endian = endian == "big";

        println!(
            "\n🎬 RUNNING COMPLETE DRIVING SCENARIO ({} steps) - {} ENDIAN",
            scenario.len(),
            endian.to_uppercase()
        );
        println!("═══════════════════════════════════════════════════════════");

        for (i, step) in scenario.iter().enumerate() {
            println!(
                "\n📍 STEP {}/{}: Processing '{}'... ({})",
                i + 1,
                scenario.len(),
                step.step_name,
                endian
            );

            // Convert to CAN messages with explicit endianness
            let can_messages = step.to_can_messages_with_endian(is_big_endian);
            println!(
                "\n📡 Converting to {} CAN messages ({} endian)...",
                can_messages.len(),
                endian
            );

            // Store CAN messages in database
            println!(
                "\n💾 Storing {} CAN messages to SQLite database...",
                can_messages.len()
            );
            store_can_messages(&pool, &can_messages).await?;

            // Wait a moment to ensure database write is committed
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;

            // Send step_name and endianness to RabbitMQ (if available)
            println!("\n📨 Sending step_data to RabbitMQ...");
            if let Some(ch) = &channel {
                match send_step_data_to_rabbitmq(ch, &step.step_name, endian).await {
                    Ok(_) => {
                        println!(
                            "   └─ Step '{}' + endian '{}' → RabbitMQ ✅",
                            step.step_name, endian
                        );
                    }
                    Err(e) => {
                        println!("   └─ ⚠️ RabbitMQ error ({}), continuing without it", e);
                    }
                }
            } else {
                println!("   └─ ⚠️ Skipping RabbitMQ (not connected)");
            }
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }

    Ok(())
}
