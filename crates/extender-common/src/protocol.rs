use serde::{Deserialize, Serialize};
use crate::events::InputEvent;

pub const EXTENDER_PROTOCOL_MAGIC: u32 = 0x4558544E; // "EXTN"
pub const EXTENDER_PROTOCOL_VERSION: u16 = 1;
pub const DEFAULT_STREAM_PORT: u16 = 8554;
pub const DEFAULT_INPUT_PORT: u16 = 8555;
pub const DEFAULT_DISCOVERY_PORT: u16 = 8556;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum VideoCodec {
    H264Vaapi = 1,
    H264Nvenc = 2,
    H264Software = 3,
    H265Vaapi = 4,
    H265Nvenc = 5,
    Vp8 = 6,
    Vp9 = 7,
    Av1 = 8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum LegacyPacketType {
    HandshakeReq = 1,
    HandshakeResp = 2,
    VideoData = 3,
    InputData = 4,
    Ping = 5,
    Pong = 6,
    Disconnect = 7,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LegacyPacketHeader {
    pub magic: u32,
    pub version: u16,
    pub packet_type: LegacyPacketType,
    pub sequence: u64,
    pub timestamp_us: u64,
    pub payload_len: u32,
    pub checksum: u32,
}

impl LegacyPacketHeader {
    pub fn new(packet_type: LegacyPacketType, sequence: u64, timestamp_us: u64, payload_len: u32, checksum: u32) -> Self {
        Self {
            magic: EXTENDER_PROTOCOL_MAGIC,
            version: EXTENDER_PROTOCOL_VERSION,
            packet_type,
            sequence,
            timestamp_us,
            payload_len,
            checksum,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HandshakeRequest {
    pub client_name: String,
    pub preferred_width: u32,
    pub preferred_height: u32,
    pub refresh_rate: u32,
    pub supported_codecs: Vec<VideoCodec>,
    pub auth_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HandshakeResponse {
    pub accepted: bool,
    pub error_message: Option<String>,
    pub selected_width: u32,
    pub selected_height: u32,
    pub selected_codec: VideoCodec,
    pub pipewire_node_id: Option<u32>,
    pub stream_port: u16,
    pub input_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PacketPayload {
    HandshakeReq(HandshakeRequest),
    HandshakeResp(HandshakeResponse),
    InputData(InputEvent),
    Ping { sequence: u64, timestamp_us: u64 },
    Pong { sequence: u64, timestamp_us: u64 },
    Disconnect,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Packet {
    pub magic: u32,
    pub version: u16,
    pub payload: PacketPayload,
}

impl Packet {
    pub fn new(payload: PacketPayload) -> Self {
        Self {
            magic: EXTENDER_PROTOCOL_MAGIC,
            version: EXTENDER_PROTOCOL_VERSION,
            payload,
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    /// Decodes both unified packets and legacy header framing
    pub fn decode(bytes: &[u8]) -> Result<(Self, bool), bincode::Error> {
        // Try unified decode first
        if let Ok(packet) = bincode::deserialize::<Packet>(bytes) {
            if packet.magic == EXTENDER_PROTOCOL_MAGIC && packet.version == EXTENDER_PROTOCOL_VERSION {
                return Ok((packet, false));
            }
        }

        // Try legacy framing decode
        let mut cursor = std::io::Cursor::new(bytes);
        if let Ok(legacy_header) = bincode::deserialize_from::<_, LegacyPacketHeader>(&mut cursor) {
            if legacy_header.magic == EXTENDER_PROTOCOL_MAGIC {
                let offset = cursor.position() as usize;
                let payload_bytes = &bytes[offset..];
                match legacy_header.packet_type {
                    LegacyPacketType::HandshakeReq => {
                        if let Ok(req) = bincode::deserialize::<HandshakeRequest>(payload_bytes) {
                            return Ok((Packet::new(PacketPayload::HandshakeReq(req)), true));
                        }
                    }
                    LegacyPacketType::HandshakeResp => {
                        if let Ok(resp) = bincode::deserialize::<HandshakeResponse>(payload_bytes) {
                            return Ok((Packet::new(PacketPayload::HandshakeResp(resp)), true));
                        }
                    }
                    LegacyPacketType::InputData => {
                        if let Ok(event) = bincode::deserialize::<InputEvent>(payload_bytes) {
                            return Ok((Packet::new(PacketPayload::InputData(event)), true));
                        }
                    }
                    LegacyPacketType::Ping => {
                        return Ok((Packet::new(PacketPayload::Ping {
                            sequence: legacy_header.sequence,
                            timestamp_us: legacy_header.timestamp_us,
                        }), true));
                    }
                    LegacyPacketType::Pong => {
                        return Ok((Packet::new(PacketPayload::Pong {
                            sequence: legacy_header.sequence,
                            timestamp_us: legacy_header.timestamp_us,
                        }), true));
                    }
                    LegacyPacketType::Disconnect => {
                        return Ok((Packet::new(PacketPayload::Disconnect), true));
                    }
                    LegacyPacketType::VideoData => {}
                }
            }
        }

        Err(bincode::ErrorKind::Custom("Failed to decode packet in unified or legacy format".to_string()).into())
    }

    pub fn encode_legacy(&self) -> Result<Vec<u8>, bincode::Error> {
        match &self.payload {
            PacketPayload::HandshakeResp(resp) => {
                let payload_bytes = bincode::serialize(resp)?;
                let header = LegacyPacketHeader::new(
                    LegacyPacketType::HandshakeResp,
                    1,
                    0,
                    payload_bytes.len() as u32,
                    crc32fast::Hasher::new().finalize(),
                );
                let mut bytes = bincode::serialize(&header)?;
                bytes.extend(payload_bytes);
                Ok(bytes)
            }
            PacketPayload::Pong { sequence, timestamp_us } => {
                let header = LegacyPacketHeader::new(
                    LegacyPacketType::Pong,
                    *sequence,
                    *timestamp_us,
                    0,
                    0,
                );
                bincode::serialize(&header)
            }
            _ => self.encode(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_legacy_handshake_roundtrip() {
        let req = HandshakeRequest {
            client_name: "ExtenderClient-Wayland".to_string(),
            preferred_width: 1920,
            preferred_height: 1080,
            refresh_rate: 60,
            supported_codecs: vec![VideoCodec::H264Software],
            auth_token: None,
        };

        let payload_bytes = bincode::serialize(&req).unwrap();
        let header = LegacyPacketHeader::new(
            LegacyPacketType::HandshakeReq,
            1,
            0,
            payload_bytes.len() as u32,
            crc32fast::Hasher::new().finalize(),
        );
        let mut packet_bytes = bincode::serialize(&header).unwrap();
        packet_bytes.extend(payload_bytes);

        let (decoded, is_legacy) = Packet::decode(&packet_bytes).expect("Failed to decode legacy packet");
        assert!(is_legacy);
        match decoded.payload {
            PacketPayload::HandshakeReq(r) => assert_eq!(r.client_name, "ExtenderClient-Wayland"),
            _ => panic!("Expected HandshakeReq"),
        }
    }

    #[test]
    fn test_unified_handshake_roundtrip() {
        let req = HandshakeRequest {
            client_name: "ExtenderClient-Wayland".to_string(),
            preferred_width: 1920,
            preferred_height: 1080,
            refresh_rate: 60,
            supported_codecs: vec![VideoCodec::H264Software],
            auth_token: None,
        };

        let packet = Packet::new(PacketPayload::HandshakeReq(req));
        let packet_bytes = packet.encode().unwrap();
        let (decoded, is_legacy) = Packet::decode(&packet_bytes).expect("Failed to decode unified packet");
        assert!(!is_legacy);
        match decoded.payload {
            PacketPayload::HandshakeReq(r) => assert_eq!(r.client_name, "ExtenderClient-Wayland"),
            _ => panic!("Expected HandshakeReq"),
        }
    }
}
