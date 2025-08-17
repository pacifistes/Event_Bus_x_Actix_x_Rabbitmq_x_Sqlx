use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct NewCanMessage {
    pub id: u16,
    pub speed: u8,
    pub temperature: u8,
    pub pressure: u16,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CanMessage {
    pub id: u16,       // ID on 11 bits (0..=0x7FF)
    pub dlc: u8,       // number of used bytes (0..=8)
    pub data: [u8; 8], // data can be 8 bytes max
    pub speed: u8,
    pub temperature: u8,
    pub pressure: u16,
    pub timestamp: String, // ISO timestamp for tracking
}

impl CanMessage {
    pub fn new(id: u16, speed: u8, temperature: u8, pressure: u16) -> Self {
        assert!(id <= 0x7FF, "ID must fit on 11 bits");
        assert!(pressure <= 0x3FF, "pressure must fit on 10 bits");

        let mut data = [0u8; 8];

        // Byte 0 = speed
        data[0] = speed;

        // Byte 1 = temperature
        data[1] = temperature;

        // Byte 2 = bits 0..7 of pressure
        data[2] = (pressure & 0xFF) as u8;

        // Byte 3 = bits 8..9 of pressure (in the 2 LSB)
        data[3] = ((pressure >> 8) & 0x03) as u8;

        Self {
            id,
            dlc: 4, // 4 bytes used
            data,
            speed,
            temperature,
            pressure,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}
