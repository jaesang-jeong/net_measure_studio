use byteorder::{BigEndian, ByteOrder};
use std::fmt;

pub const SYNC_WORD: u16 = 0xAA55;
pub const TAIL_WORD: u16 = 0x55AA;
pub const HEADER_SIZE: usize = 20; // Sync(2) + DevId(2) + Seq(4) + Time(8) + Status(2) + ChCount(2)
pub const FOOTER_SIZE: usize = 4;  // CRC(2) + Tail(2)
#[allow(dead_code)]
pub const DEFAULT_CHANNEL_COUNT: usize = 100;
#[allow(dead_code)]
pub const DEFAULT_PACKET_SIZE: usize = HEADER_SIZE + DEFAULT_CHANNEL_COUNT * 4 + FOOTER_SIZE; // 424 bytes

#[derive(Debug, Clone, PartialEq)]
pub enum PacketError {
    IncompletePacket { expected: usize, actual: usize },
    InvalidSyncWord(u16),
    InvalidTailWord(u16),
    CrcMismatch { expected: u16, actual: u16 },
    BufferTooShort,
}

impl fmt::Display for PacketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PacketError::IncompletePacket { expected, actual } => {
                write!(f, "Incomplete packet: expected {} bytes, got {}", expected, actual)
            }
            PacketError::InvalidSyncWord(w) => write!(f, "Invalid sync word: 0x{:04X}", w),
            PacketError::InvalidTailWord(w) => write!(f, "Invalid tail word: 0x{:04X}", w),
            PacketError::CrcMismatch { expected, actual } => {
                write!(f, "CRC mismatch: expected 0x{:04X}, calculated 0x{:04X}", expected, actual)
            }
            PacketError::BufferTooShort => write!(f, "Buffer is too short for packet header"),
        }
    }
}

impl std::error::Error for PacketError {}

/// Standard CRC-16/CCITT-FALSE (poly 0x1021, init 0xFFFF, no reflect, xorout 0x0000)
pub fn calc_crc16_ccitt(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            if (crc & 0x8000) != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

/// CRC-16/AUG-CCITT (poly 0x1021, init 0x1D0F, no reflect, xorout 0x0000) - LabVIEW 기본 규격 호환
pub fn calc_crc16_aug_ccitt(data: &[u8]) -> u16 {
    let mut crc: u16 = 0x1D0F;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            if (crc & 0x8000) != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeasurementPacket {
    pub device_id: u16,
    pub sequence: u32,
    pub timestamp: u64, // Unix Epoch ms
    pub status_flags: u16,
    pub channel_count: u16,
    pub data: Vec<f32>,
}

impl MeasurementPacket {
    pub fn new(device_id: u16, sequence: u32, timestamp: u64, status_flags: u16, data: Vec<f32>) -> Self {
        let channel_count = data.len() as u16;
        Self {
            device_id,
            sequence,
            timestamp,
            status_flags,
            channel_count,
            data,
        }
    }

    /// Calculate total packet size in bytes
    pub fn packet_size(&self) -> usize {
        HEADER_SIZE + (self.channel_count as usize) * 4 + FOOTER_SIZE
    }

    /// Serialize packet into Big-Endian binary buffer
    pub fn to_bytes(&self) -> Vec<u8> {
        let size = self.packet_size();
        let mut buf = vec![0u8; size];

        BigEndian::write_u16(&mut buf[0x00..0x02], SYNC_WORD);
        BigEndian::write_u16(&mut buf[0x02..0x04], self.device_id);
        BigEndian::write_u32(&mut buf[0x04..0x08], self.sequence);
        BigEndian::write_u64(&mut buf[0x08..0x10], self.timestamp);
        BigEndian::write_u16(&mut buf[0x10..0x12], self.status_flags);
        BigEndian::write_u16(&mut buf[0x12..0x14], self.channel_count);

        let mut offset = 0x14;
        for &val in &self.data {
            BigEndian::write_f32(&mut buf[offset..offset + 4], val);
            offset += 4;
        }

        // CRC over Sync Word up to Data Payload
        let crc = calc_crc16_ccitt(&buf[0..offset]);
        BigEndian::write_u16(&mut buf[offset..offset + 2], crc);
        BigEndian::write_u16(&mut buf[offset + 2..offset + 4], TAIL_WORD);

        buf
    }

    /// Deserialize from a raw byte slice (defaults to CRC verification enabled)
    #[allow(dead_code)]
    pub fn from_bytes(data: &[u8]) -> Result<Self, PacketError> {
        Self::from_bytes_opt(data, true)
    }

    /// Deserialize from a raw byte slice with optional CRC-16 check
    pub fn from_bytes_opt(data: &[u8], verify_crc: bool) -> Result<Self, PacketError> {
        if data.len() < HEADER_SIZE + FOOTER_SIZE {
            return Err(PacketError::BufferTooShort);
        }

        let sync = BigEndian::read_u16(&data[0x00..0x02]);
        if sync != SYNC_WORD {
            return Err(PacketError::InvalidSyncWord(sync));
        }

        let device_id = BigEndian::read_u16(&data[0x02..0x04]);
        let sequence = BigEndian::read_u32(&data[0x04..0x08]);
        let timestamp = BigEndian::read_u64(&data[0x08..0x10]);
        let status_flags = BigEndian::read_u16(&data[0x10..0x12]);
        let channel_count = BigEndian::read_u16(&data[0x12..0x14]);

        let payload_len = (channel_count as usize) * 4;
        let expected_total_size = HEADER_SIZE + payload_len + FOOTER_SIZE;

        if data.len() < expected_total_size {
            return Err(PacketError::IncompletePacket {
                expected: expected_total_size,
                actual: data.len(),
            });
        }

        let crc_offset = HEADER_SIZE + payload_len;
        let tail_offset = crc_offset + 2;

        let expected_crc = BigEndian::read_u16(&data[crc_offset..crc_offset + 2]);
        let tail = BigEndian::read_u16(&data[tail_offset..tail_offset + 2]);

        if tail != TAIL_WORD {
            return Err(PacketError::InvalidTailWord(tail));
        }

        // Validate CRC-16 (옵션이 켜져 있을 때만 검증, CCITT-FALSE(0xFFFF) 또는 LabVIEW 기본 AUG-CCITT(0x1D0F) 모두 허용)
        if verify_crc {
            let calculated_crc = calc_crc16_ccitt(&data[0..crc_offset]);
            let calculated_crc_aug = calc_crc16_aug_ccitt(&data[0..crc_offset]);
            if expected_crc != calculated_crc && expected_crc != calculated_crc_aug {
                return Err(PacketError::CrcMismatch {
                    expected: expected_crc,
                    actual: calculated_crc,
                });
            }
        }

        // Extract float values
        let mut float_data = Vec::with_capacity(channel_count as usize);
        let mut offset = HEADER_SIZE;
        for _ in 0..channel_count {
            float_data.push(BigEndian::read_f32(&data[offset..offset + 4]));
            offset += 4;
        }

        Ok(Self {
            device_id,
            sequence,
            timestamp,
            status_flags,
            channel_count,
            data: float_data,
        })
    }
}

/// TCP Stream framing parser: Handles MTU-exceeding fragmentation and stream reassembly
#[allow(dead_code)]
pub fn extract_packet_from_stream(buffer: &mut Vec<u8>) -> Result<Option<MeasurementPacket>, PacketError> {
    extract_packet_from_stream_opt(buffer, true)
}

/// TCP Stream framing parser with optional CRC check
pub fn extract_packet_from_stream_opt(
    buffer: &mut Vec<u8>,
    verify_crc: bool,
) -> Result<Option<MeasurementPacket>, PacketError> {
    loop {
        if buffer.len() < HEADER_SIZE + FOOTER_SIZE {
            return Ok(None);
        }

        // Look for Sync Word 0xAA55
        let mut sync_index = None;
        for i in 0..buffer.len() - 1 {
            if buffer[i] == 0xAA && buffer[i + 1] == 0x55 {
                sync_index = Some(i);
                break;
            }
        }

        let sync_pos = match sync_index {
            Some(pos) => pos,
            None => {
                // If sync word not found, retain last 1 byte (might be partial 0xAA)
                let drop_len = if !buffer.is_empty() && buffer[buffer.len() - 1] == 0xAA {
                    buffer.len() - 1
                } else {
                    buffer.len()
                };
                buffer.drain(0..drop_len);
                return Ok(None);
            }
        };

        // Discard any garbage bytes before sync word
        if sync_pos > 0 {
            buffer.drain(0..sync_pos);
        }

        if buffer.len() < HEADER_SIZE {
            return Ok(None);
        }

        let channel_count = BigEndian::read_u16(&buffer[0x12..0x14]);
        let total_size = HEADER_SIZE + (channel_count as usize) * 4 + FOOTER_SIZE;

        if buffer.len() < total_size {
            // Wait for more data in TCP stream
            return Ok(None);
        }

        // Attempt to parse
        match MeasurementPacket::from_bytes_opt(&buffer[0..total_size], verify_crc) {
            Ok(pkt) => {
                buffer.drain(0..total_size);
                return Ok(Some(pkt));
            }
            Err(_) => {
                // Corrupted packet or false sync word: drop sync word and retry search
                buffer.drain(0..2);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_serialization_and_deserialization() {
        let values: Vec<f32> = (0..100).map(|i| i as f32 * 1.25).collect();
        let pkt = MeasurementPacket::new(1, 42, 1700000000000, 0x0001, values.clone());
        let bytes = pkt.to_bytes();

        assert_eq!(bytes.len(), 424);

        let parsed = MeasurementPacket::from_bytes(&bytes).expect("Failed to parse packet");
        assert_eq!(parsed.device_id, 1);
        assert_eq!(parsed.sequence, 42);
        assert_eq!(parsed.timestamp, 1700000000000);
        assert_eq!(parsed.status_flags, 0x0001);
        assert_eq!(parsed.channel_count, 100);
        assert_eq!(parsed.data, values);
    }

    #[test]
    fn test_large_packet_over_mtu() {
        // 500 channels = 20 + 2000 + 4 = 2024 bytes (exceeds Ethernet MTU 1500)
        let values: Vec<f32> = (0..500).map(|i| i as f32 * 0.5).collect();
        let pkt = MeasurementPacket::new(2, 100, 1700000001000, 0, values.clone());
        let bytes = pkt.to_bytes();
        assert_eq!(bytes.len(), 2024);

        let mut stream_buf = Vec::new();
        // Simulate fragmented TCP delivery
        stream_buf.extend_from_slice(&bytes[0..1000]);
        let p1 = extract_packet_from_stream(&mut stream_buf).unwrap();
        assert!(p1.is_none());

        stream_buf.extend_from_slice(&bytes[1000..]);
        let p2 = extract_packet_from_stream(&mut stream_buf).unwrap();
        assert!(p2.is_some());
        assert_eq!(p2.unwrap().channel_count, 500);
        assert!(stream_buf.is_empty());
    }

    #[test]
    fn test_crc_options_and_labview_compatibility() {
        // 1. Verify standard test vector "123456789"
        assert_eq!(calc_crc16_ccitt(b"123456789"), 0x29B1);
        assert_eq!(calc_crc16_aug_ccitt(b"123456789"), 0xE5CC);

        // 2. Create valid packet and test CRC bypass
        let values: Vec<f32> = vec![1.0, 2.0, 3.0];
        let pkt = MeasurementPacket::new(1, 1, 1000, 0, values);
        let mut bytes = pkt.to_bytes();

        // Intentionally corrupt the CRC bytes
        let crc_pos = bytes.len() - 4;
        bytes[crc_pos] = 0xDE;
        bytes[crc_pos + 1] = 0xAD;

        // When verify_crc = true, corrupted CRC must fail
        assert!(MeasurementPacket::from_bytes_opt(&bytes, true).is_err());

        // When verify_crc = false, corrupted CRC must pass!
        let parsed_bypass = MeasurementPacket::from_bytes_opt(&bytes, false);
        assert!(parsed_bypass.is_ok());

        // 3. Test LabVIEW default AUG-CCITT CRC (0x1D0F init)
        let aug_crc = calc_crc16_aug_ccitt(&bytes[0..crc_pos]);
        BigEndian::write_u16(&mut bytes[crc_pos..crc_pos + 2], aug_crc);

        // LabVIEW AUG-CCITT CRC is accepted even with verify_crc = true!
        let parsed_aug = MeasurementPacket::from_bytes_opt(&bytes, true);
        assert!(parsed_aug.is_ok());
    }
}
