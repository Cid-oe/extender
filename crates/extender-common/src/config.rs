use serde::{Deserialize, Serialize};
use crate::protocol::VideoCodec;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub width: u32,
    pub height: u32,
    pub refresh_rate: u32,
    pub bitrate_kbps: u32,
    pub codec: VideoCodec,
    pub stream_port: u16,
    pub input_port: u16,
    pub enable_hw_accel: bool,
    pub pin: Option<String>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            refresh_rate: 60,
            bitrate_kbps: 8000,
            codec: VideoCodec::H264Vaapi,
            stream_port: 8554,
            input_port: 8555,
            enable_hw_accel: true,
            pin: None,
        }
    }
}
