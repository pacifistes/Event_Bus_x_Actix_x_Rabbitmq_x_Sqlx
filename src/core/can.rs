use serde::{Deserialize, Serialize};

/// Unified CAN message structure for all uses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanMessage {
    pub id: u16,           // CAN ID on 11 bits (0..=0x7FF)
    pub dlc: u8,           // Data Length Code - number of used bytes (0..=8)
    pub data: [u8; 8],     // CAN data payload (max 8 bytes)
    pub timestamp: String, // ISO timestamp for tracking
}

impl CanMessage {
    /// Extract bits from a byte array starting at a specific bit position
    ///
    /// # Arguments
    /// * `data` - The byte array to extract bits from
    /// * `start_bit` - The starting bit position (0-based)
    /// * `num_bits` - The number of bits to extract (max 64)
    ///
    /// # Returns
    /// The extracted bits as a u64 value
    pub fn extract_bits_from_bytes(data: &[u8], start_bit: usize, num_bits: usize) -> u64 {
        if num_bits == 0 || num_bits > 64 {
            return 0;
        }

        let start_byte = start_bit / 8;
        let start_bit_in_byte = start_bit % 8;
        let mut result = 0u64;
        let mut bits_read = 0;

        for byte_idx in start_byte..data.len() {
            if bits_read >= num_bits {
                break;
            }

            let current_byte = data[byte_idx];
            let bits_to_read_from_byte = if byte_idx == start_byte {
                (8 - start_bit_in_byte).min(num_bits - bits_read)
            } else {
                (num_bits - bits_read).min(8)
            };

            let shift_in_byte = if byte_idx == start_byte {
                start_bit_in_byte
            } else {
                0
            };

            let mask = (1u8 << bits_to_read_from_byte) - 1;
            let extracted_bits = (current_byte >> shift_in_byte) & mask;

            result |= (extracted_bits as u64) << bits_read;
            bits_read += bits_to_read_from_byte;
        }

        result
    }

    /// Set bits in a byte array starting at a specific bit position
    ///
    /// # Arguments
    /// * `data` - The mutable byte array to modify
    /// * `start_bit` - The starting bit position (0-based)
    /// * `num_bits` - The number of bits to set (max 64)
    /// * `value` - The value to set in the specified bits
    pub fn set_bits_in_bytes(data: &mut [u8], start_bit: usize, num_bits: usize, value: u64) {
        if num_bits == 0 || num_bits > 64 {
            return;
        }

        let start_byte = start_bit / 8;
        let start_bit_in_byte = start_bit % 8;
        let mut bits_written = 0;

        for byte_idx in start_byte..data.len() {
            if bits_written >= num_bits {
                break;
            }

            let bits_to_write_to_byte = if byte_idx == start_byte {
                (8 - start_bit_in_byte).min(num_bits - bits_written)
            } else {
                (num_bits - bits_written).min(8)
            };

            let shift_in_byte = if byte_idx == start_byte {
                start_bit_in_byte
            } else {
                0
            };

            let mask = ((1u8 << bits_to_write_to_byte) - 1) << shift_in_byte;
            let value_bits = ((value >> bits_written) as u8) << shift_in_byte;

            data[byte_idx] = (data[byte_idx] & !mask) | (value_bits & mask);
            bits_written += bits_to_write_to_byte;
        }
    }
}
