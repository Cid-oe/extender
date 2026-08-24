pub mod config;
pub mod events;
pub mod protocol;

pub use config::SessionConfig;
pub use events::{InputEvent, KeyAction, MouseButton, MouseEvent};
pub use protocol::{
    HandshakeRequest, HandshakeResponse, Packet, PacketPayload, VideoCodec,
    DEFAULT_DISCOVERY_PORT, DEFAULT_INPUT_PORT, DEFAULT_STREAM_PORT,
    EXTENDER_PROTOCOL_MAGIC, EXTENDER_PROTOCOL_VERSION,
};
