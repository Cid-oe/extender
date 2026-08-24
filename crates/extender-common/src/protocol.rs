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

    pub fn decode(bytes: &[u8]) -> Result<Self, bincode::Error> {
        let packet: Self = bincode::deserialize(bytes)?;
        if packet.magic != EXTENDER_PROTOCOL_MAGIC || packet.version != EXTENDER_PROTOCOL_VERSION {
            return Err(bincode::ErrorKind::Custom("Invalid protocol magic or version".to_string()).into());
        }
        Ok(packet)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_handshake_roundtrip() {
        let req = HandshakeRequest {
            client_name: "TestLaptop".to_string(),
            preferred_width: 1920,
            preferred_height: 1080,
            refresh_rate: 60,
            supported_codecs: vec![VideoCodec::H264Software],
            auth_token: None,
        };

        let packet = Packet::new(PacketPayload::HandshakeReq(req.clone()));
        let encoded = packet.encode().expect("Failed to encode");
        let decoded = Packet::decode(&encoded).expect("Failed to decode");

        match decoded.payload {
            PacketPayload::HandshakeReq(decoded_req) => assert_eq!(req, decoded_req),
            _ => panic!("Expected HandshakeReq payload"),
        }
    }

    #[test]
    fn test_packet_response_roundtrip() {
        let resp = HandshakeResponse {
            accepted: true,
            error_message: None,
            selected_width: 1920,
            selected_height: 1080,
            selected_codec: VideoCodec::H264Software,
            pipewire_node_id: Some(42),
            stream_port: 8554,
            input_port: 8555,
        };

        let packet = Packet::new(PacketPayload::HandshakeResp(resp.clone()));
        let encoded = packet.encode().expect("Failed to encode");
        let decoded = Packet::decode(&encoded).expect("Failed to decode");

        match decoded.payload {
            PacketPayload::HandshakeResp(decoded_resp) => assert_eq!(resp, decoded_resp),
            _ => panic!("Expected HandshakeResp payload"),
        }
    }
}
