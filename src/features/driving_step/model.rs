use serde::{Deserialize, Serialize};

use crate::core::can::CanMessage;

/// Realistic engine data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineData {
    pub rpm: u16,             // Engine RPM
    pub coolant_temp: i16,    // Coolant temperature in °C (-40 to +215)
    pub throttle_pos: u8,     // Throttle position (0-100%)
    pub engine_load: u8,      // Engine load percentage
    pub intake_temp: i16,     // Intake air temperature in °C
    pub fuel_pressure: u16,   // Fuel pressure in kPa
    pub engine_running: bool, // Engine status
}

/// Vehicle speed and transmission data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehicleSpeedData {
    pub vehicle_speed: f32,     // Speed in km/h
    pub gear_position: u8,      // Current gear (0=Park, 1-6=gears, 15=Reverse)
    pub wheel_speeds: [f32; 4], // Individual wheel speeds [FL, FR, RL, RR]
    pub abs_active: bool,       // ABS system status
    pub traction_control: bool, // Traction control status
    pub cruise_control: bool,   // Cruise control status
}

/// Climate control data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClimateData {
    pub cabin_temp: i16,         // Cabin temperature in °C (-40 to +85)
    pub target_temp: i16,        // Target temperature in °C
    pub outside_temp: i16,       // Outside temperature in °C
    pub fan_speed: u8,           // Fan speed (0-255)
    pub ac_compressor: bool,     // AC compressor status
    pub heater: bool,            // Heater status
    pub defrost: bool,           // Defrost status
    pub auto_mode: bool,         // Auto climate mode
    pub air_recirculation: bool, // Air recirculation mode
}

/// Complete driving step with all vehicle data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrivingStep {
    pub step_name: String,
    pub engine: EngineData,
    pub speed: VehicleSpeedData,
    pub climate: ClimateData,
    pub duration_ms: u64,
}

impl DrivingStep {
    // CAN ID assignments for different parts of DrivingStep
    const ENGINE_RPM_CAN_ID: u16 = 0x100;
    const ENGINE_TEMP_CAN_ID: u16 = 0x101;

    const SPEED_DATA_CAN_ID: u16 = 0x200;
    const SPEED_FLAGS_CAN_ID: u16 = 0x201;
    const CLIMATE_TEMP_CAN_ID: u16 = 0x300;
    const CLIMATE_FAN_CAN_ID: u16 = 0x301;
    const STEP_INFO_CAN_ID: u16 = 0x400;

    /// Convert DrivingStep to multiple CAN messages
    pub fn to_can_messages(&self) -> Vec<CanMessage> {
        let mut messages = Vec::new();
        let timestamp = chrono::Utc::now().to_rfc3339();

        // Engine RPM and related data
        let mut engine_rpm_data = [0u8; 8];
        engine_rpm_data[0] = (self.engine.rpm & 0xFF) as u8;
        engine_rpm_data[1] = ((self.engine.rpm >> 8) & 0xFF) as u8;
        engine_rpm_data[2] = ((self.engine.fuel_pressure / 10) & 0xFF) as u8;
        engine_rpm_data[3] = ((self.engine.fuel_pressure / 10 >> 8) & 0xFF) as u8;
        engine_rpm_data[4] = if self.engine.engine_running { 1 } else { 0 };

        messages.push(CanMessage {
            id: Self::ENGINE_RPM_CAN_ID,
            dlc: 5,
            data: engine_rpm_data,
            timestamp: timestamp.clone(),
        });

        // Engine temperature data
        let mut engine_temp_data = [0u8; 8];
        engine_temp_data[0] = ((self.engine.coolant_temp + 40).max(0).min(255)) as u8;
        engine_temp_data[1] = ((self.engine.intake_temp + 40).max(0).min(255)) as u8;
        engine_temp_data[2] = self.engine.throttle_pos;
        engine_temp_data[3] = self.engine.engine_load;

        messages.push(CanMessage {
            id: Self::ENGINE_TEMP_CAN_ID,
            dlc: 4,
            data: engine_temp_data,
            timestamp: timestamp.clone(),
        });

        // Vehicle speed and gear data
        let mut speed_data = [0u8; 8];
        let speed_encoded = (self.speed.vehicle_speed * 10.0).min(6553.5) as u16;
        speed_data[0] = (speed_encoded & 0xFF) as u8;
        speed_data[1] = ((speed_encoded >> 8) & 0xFF) as u8;
        speed_data[2] = self.speed.gear_position;

        // Wheel speeds (simplified, 1 byte each)
        for (i, &wheel_speed) in self.speed.wheel_speeds.iter().enumerate().take(4) {
            speed_data[3 + i] = wheel_speed.min(255.0) as u8;
        }

        messages.push(CanMessage {
            id: Self::SPEED_DATA_CAN_ID,
            dlc: 7,
            data: speed_data,
            timestamp: timestamp.clone(),
        });

        // Speed flags (ABS, traction control, etc.)
        let mut speed_flags_data = [0u8; 8];
        let mut flags = 0u8;
        if self.speed.abs_active {
            flags |= 0x01;
        }
        if self.speed.traction_control {
            flags |= 0x02;
        }
        if self.speed.cruise_control {
            flags |= 0x04;
        }
        speed_flags_data[0] = flags;

        messages.push(CanMessage {
            id: Self::SPEED_FLAGS_CAN_ID,
            dlc: 1,
            data: speed_flags_data,
            timestamp: timestamp.clone(),
        });

        // Climate temperature data
        let mut climate_temp_data = [0u8; 8];
        climate_temp_data[0] = ((self.climate.cabin_temp + 40).max(0).min(255)) as u8;
        climate_temp_data[1] = ((self.climate.target_temp + 40).max(0).min(255)) as u8;
        climate_temp_data[2] = ((self.climate.outside_temp + 40).max(0).min(255)) as u8;

        messages.push(CanMessage {
            id: Self::CLIMATE_TEMP_CAN_ID,
            dlc: 3,
            data: climate_temp_data,
            timestamp: timestamp.clone(),
        });

        // Climate fan and flags data
        let mut climate_fan_data = [0u8; 8];
        climate_fan_data[0] = self.climate.fan_speed;
        let mut climate_flags = 0u8;
        if self.climate.ac_compressor {
            climate_flags |= 0x01;
        }
        if self.climate.heater {
            climate_flags |= 0x02;
        }
        if self.climate.defrost {
            climate_flags |= 0x04;
        }
        if self.climate.auto_mode {
            climate_flags |= 0x08;
        }
        if self.climate.air_recirculation {
            climate_flags |= 0x10;
        }
        climate_fan_data[1] = climate_flags;

        messages.push(CanMessage {
            id: Self::CLIMATE_FAN_CAN_ID,
            dlc: 2,
            data: climate_fan_data,
            timestamp: timestamp.clone(),
        });

        // Step info (duration, step name hash for verification)
        let mut step_info_data = [0u8; 8];
        let duration_bytes = (self.duration_ms as u32).to_le_bytes();
        step_info_data[0..4].copy_from_slice(&duration_bytes);

        // Simple hash of step name for verification
        let step_name_hash = self
            .step_name
            .bytes()
            .fold(0u32, |acc, b| acc.wrapping_add(b as u32));
        let hash_bytes = step_name_hash.to_le_bytes();
        step_info_data[4..8].copy_from_slice(&hash_bytes);

        messages.push(CanMessage {
            id: Self::STEP_INFO_CAN_ID,
            dlc: 8,
            data: step_info_data,
            timestamp: timestamp.clone(),
        });

        messages
    }

    /// Reconstruct DrivingStep from multiple CAN messages
    pub fn from_can_messages(messages: &[CanMessage], step_name: String) -> Result<Self, String> {
        let mut engine_data = None;
        let mut engine_temp_data = None;
        let mut speed_data = None;
        let mut speed_flags_data = None;
        let mut climate_temp_data = None;
        let mut climate_fan_data = None;
        let mut step_info_data = None;

        // Parse messages by CAN ID
        for msg in messages {
            match msg.id {
                Self::ENGINE_RPM_CAN_ID => {
                    if msg.dlc >= 5 {
                        let rpm = (msg.data[1] as u16) << 8 | (msg.data[0] as u16);
                        let fuel_pressure = ((msg.data[3] as u16) << 8 | (msg.data[2] as u16)) * 10;
                        let engine_running = msg.data[4] != 0;
                        engine_data = Some((rpm, fuel_pressure, engine_running));
                    }
                }
                Self::ENGINE_TEMP_CAN_ID => {
                    if msg.dlc >= 4 {
                        let coolant_temp = msg.data[0] as i16 - 40;
                        let intake_temp = msg.data[1] as i16 - 40;
                        let throttle_pos = msg.data[2];
                        let engine_load = msg.data[3];
                        engine_temp_data =
                            Some((coolant_temp, intake_temp, throttle_pos, engine_load));
                    }
                }
                Self::SPEED_DATA_CAN_ID => {
                    if msg.dlc >= 7 {
                        let speed_raw = (msg.data[1] as u16) << 8 | (msg.data[0] as u16);
                        let vehicle_speed = speed_raw as f32 / 10.0;
                        let gear_position = msg.data[2];
                        let wheel_speeds = [
                            msg.data[3] as f32,
                            msg.data[4] as f32,
                            msg.data[5] as f32,
                            msg.data[6] as f32,
                        ];
                        speed_data = Some((vehicle_speed, gear_position, wheel_speeds));
                    }
                }
                Self::SPEED_FLAGS_CAN_ID => {
                    if msg.dlc >= 1 {
                        let flags = msg.data[0];
                        let abs_active = (flags & 0x01) != 0;
                        let traction_control = (flags & 0x02) != 0;
                        let cruise_control = (flags & 0x04) != 0;
                        speed_flags_data = Some((abs_active, traction_control, cruise_control));
                    }
                }
                Self::CLIMATE_TEMP_CAN_ID => {
                    if msg.dlc >= 3 {
                        let cabin_temp = msg.data[0] as i16 - 40;
                        let target_temp = msg.data[1] as i16 - 40;
                        let outside_temp = msg.data[2] as i16 - 40;
                        climate_temp_data = Some((cabin_temp, target_temp, outside_temp));
                    }
                }
                Self::CLIMATE_FAN_CAN_ID => {
                    if msg.dlc >= 2 {
                        let fan_speed = msg.data[0];
                        let flags = msg.data[1];
                        let ac_compressor = (flags & 0x01) != 0;
                        let heater = (flags & 0x02) != 0;
                        let defrost = (flags & 0x04) != 0;
                        let auto_mode = (flags & 0x08) != 0;
                        let air_recirculation = (flags & 0x10) != 0;
                        climate_fan_data = Some((
                            fan_speed,
                            ac_compressor,
                            heater,
                            defrost,
                            auto_mode,
                            air_recirculation,
                        ));
                    }
                }
                Self::STEP_INFO_CAN_ID => {
                    if msg.dlc >= 8 {
                        let duration_bytes = [msg.data[0], msg.data[1], msg.data[2], msg.data[3]];
                        let duration_ms = u32::from_le_bytes(duration_bytes) as u64;
                        step_info_data = Some(duration_ms);
                    }
                }
                _ => {} // Unknown CAN ID, ignore
            }
        }

        // Verify we have all required data
        let (rpm, fuel_pressure, engine_running) = engine_data.ok_or("Missing engine RPM data")?;
        let (coolant_temp, intake_temp, throttle_pos, engine_load) =
            engine_temp_data.ok_or("Missing engine temperature data")?;
        let (vehicle_speed, gear_position, wheel_speeds) =
            speed_data.ok_or("Missing speed data")?;
        let (abs_active, traction_control, cruise_control) =
            speed_flags_data.ok_or("Missing speed flags data")?;
        let (cabin_temp, target_temp, outside_temp) =
            climate_temp_data.ok_or("Missing climate temperature data")?;
        let (fan_speed, ac_compressor, heater, defrost, auto_mode, air_recirculation) =
            climate_fan_data.ok_or("Missing climate fan data")?;
        let duration_ms = step_info_data.ok_or("Missing step info data")?;

        Ok(DrivingStep {
            step_name,
            engine: EngineData {
                rpm,
                coolant_temp,
                throttle_pos,
                engine_load,
                intake_temp,
                fuel_pressure,
                engine_running,
            },
            speed: VehicleSpeedData {
                vehicle_speed,
                gear_position,
                wheel_speeds,
                abs_active,
                traction_control,
                cruise_control,
            },
            climate: ClimateData {
                cabin_temp,
                target_temp,
                outside_temp,
                fan_speed,
                ac_compressor,
                heater,
                defrost,
                auto_mode,
                air_recirculation,
            },
            duration_ms,
        })
    }

    pub fn print_status(&self) {
        println!("\n🚗 {} 🚗", self.step_name);
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        // Engine display
        println!("🔧 ENGINE:");
        println!("   • RPM: {} rpm", self.engine.rpm);
        println!("   • Temperature: {}°C", self.engine.coolant_temp);
        println!("   • Throttle: {}%", self.engine.throttle_pos);
        println!("   • Load: {}%", self.engine.engine_load);
        println!("   • Intake Temp: {}°C", self.engine.intake_temp);
        println!("   • Fuel Pressure: {} kPa", self.engine.fuel_pressure);
        println!(
            "   • Running: {}",
            if self.engine.engine_running {
                "✅ YES"
            } else {
                "❌ NO"
            }
        );

        // Speed display
        println!("\n🏃 SPEED & TRANSMISSION:");
        println!("   • Speed: {:.1} km/h", self.speed.vehicle_speed);
        println!(
            "   • Gear: {}",
            match self.speed.gear_position {
                0 => "P (Park)".to_string(),
                1..=6 => format!("{}st/nd/rd/th", self.speed.gear_position),
                15 => "R (Reverse)".to_string(),
                _ => "Unknown".to_string(),
            }
        );
        println!(
            "   • Wheel speeds: FL={:.1}, FR={:.1}, RL={:.1}, RR={:.1} km/h",
            self.speed.wheel_speeds[0],
            self.speed.wheel_speeds[1],
            self.speed.wheel_speeds[2],
            self.speed.wheel_speeds[3]
        );
        println!(
            "   • ABS: {}",
            if self.speed.abs_active {
                "🔴 ACTIVE"
            } else {
                "⚪ INACTIVE"
            }
        );
        println!(
            "   • Traction Control: {}",
            if self.speed.traction_control {
                "🔴 ON"
            } else {
                "⚪ OFF"
            }
        );
        println!(
            "   • Cruise Control: {}",
            if self.speed.cruise_control {
                "🔴 ON"
            } else {
                "⚪ OFF"
            }
        );

        // Climate display
        println!("\n🌡️ CLIMATE CONTROL:");
        println!("   • Cabin: {}°C", self.climate.cabin_temp);
        println!("   • Target: {}°C", self.climate.target_temp);
        println!("   • Outside: {}°C", self.climate.outside_temp);
        println!("   • Fan Speed: {}/255", self.climate.fan_speed);
        println!(
            "   • AC: {}",
            if self.climate.ac_compressor {
                "❄️ ON"
            } else {
                "⚪ OFF"
            }
        );
        println!(
            "   • Heater: {}",
            if self.climate.heater {
                "🔥 ON"
            } else {
                "⚪ OFF"
            }
        );
        println!(
            "   • Defrost: {}",
            if self.climate.defrost {
                "💨 ON"
            } else {
                "⚪ OFF"
            }
        );
        println!(
            "   • Auto Mode: {}",
            if self.climate.auto_mode {
                "🤖 ON"
            } else {
                "👤 MANUAL"
            }
        );

        println!("\n⏱️ Duration: {}ms", self.duration_ms);
    }

    pub fn show_can_messages(&self) {
        let can_messages = self.to_can_messages();

        println!("\n📡 CAN MESSAGES ({} total):", can_messages.len());
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        for (i, msg) in can_messages.iter().enumerate() {
            println!("🔌 CAN Message {}:", i + 1);
            println!("   • ID: 0x{:03X}", msg.id);
            println!("   • DLC: {}", msg.dlc);
            println!("   • Data: {:02X?}", &msg.data[..msg.dlc as usize]);
            println!(
                "   • Purpose: {}",
                match msg.id {
                    0x100 => "Engine RPM + Fuel Pressure + Running status",
                    0x101 => "Engine temperatures + Throttle + Load",
                    0x200 => "Vehicle speed + Gear + Wheel speeds",
                    0x201 => "Speed flags (ABS, Traction, Cruise)",
                    0x300 => "Climate temperatures",
                    0x301 => "Climate fan + flags",
                    0x400 => "Step info (duration + name hash)",
                    _ => "Unknown",
                }
            );
            if i < can_messages.len() - 1 {
                println!("   ├─────────────────────────────────────────");
            }
        }
        println!("   └─────────────────────────────────────────");
    }
}
