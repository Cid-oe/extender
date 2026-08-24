use serde::{Deserialize, Serialize};

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
pub enum PacketType {
    HandshakeReq = 1,
    HandshakeResp = 2,
    VideoData = 3,
    InputData = 4,
    Ping = 5,
    Pong = 6,
    Disconnect = 7,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PacketHeader {
    pub magic: u32,
    pub version: u16,
    pub packet_type: PacketType,
    pub sequence: u64,
    pub timestamp_us: u64,
    pub payload_len: u32,
    pub checksum: u32,
}

impl PacketHeader {
    pub fn new(packet_type: PacketType, sequence: u64, timestamp_us: u64, payload_len: u32, checksum: u32) -> Self {
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

    pub fn validate(&self) -> bool {
        self.magic == EXTENDER_PROTOCOL_MAGIC && self.version == EXTENDER_PROTOCOL_VERSION
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HandshakeRequest {
    pub client_name: String,
    pub preferred_width: u32,
    pub preferred_height: u32,
    pub refresh_rate: u32,
    pub supported_codecs: Vec<VideoCodec>,
    pub auth_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_header_validation() {
        let header = PacketHeader::new(PacketType::HandshakeReq, 1, 1000, 64, 0);
        assert!(header.validate());

        let mut bad_header = header.clone();
        bad_header.magic = 0x1234;
        assert!(!bad_header.validate());
    }

    #[test]
    fn test_handshake_serialization() {
        let req = HandshakeRequest {
            client_name: "TestLaptop".to_string(),
            preferred_width: 1920,
            preferred_height: 1080,
            refresh_rate: 60,
            supported_codecs: vec![VideoCodec::H264Vaapi, VideoCodec::H264Software],
            auth_token: Some("secret123".to_string()),
        };

        let bytes = bincode::serialize(&req).expect("Failed to serialize");
        let decoded: HandshakeRequest = bincode::deserialize(&bytes).expect("Failed to deserialize");
        assert_eq!(req, decoded);
    }
}
