use anyhow::{Context, Result};
use extender_common::protocol::VideoCodec;
use std::process::{Child, Command, Stdio};
use tracing::info;

pub struct VideoDisplayPipeline {
    child: Option<Child>,
    pub codec: VideoCodec,
    pub stream_port: u16,
}

impl VideoDisplayPipeline {
    pub fn new(codec: VideoCodec, stream_port: u16) -> Self {
        Self {
            child: None,
            codec,
            stream_port,
        }
    }

    /// Spawns a low-latency GStreamer pipeline receiving RTP, decoding with HW or SW, and rendering on Wayland
    pub fn start_display(&mut self) -> Result<()> {
        let (depay_element, dec_element) = match self.codec {
            VideoCodec::H264Vaapi => ("rtph264depay ! h264parse", "vaapih264dec"),
            VideoCodec::H264Nvenc => ("rtph264depay ! h264parse", "nvh264dec"),
            VideoCodec::H264Software => ("rtph264depay ! h264parse", "avdec_h264"),
            VideoCodec::H265Vaapi => ("rtph265depay ! h265parse", "vaapih265dec"),
            VideoCodec::H265Nvenc => ("rtph265depay ! h265parse", "nvh265dec"),
            VideoCodec::Vp8 => ("rtpvp8depay", "vp8dec"),
            VideoCodec::Vp9 => ("rtpvp9depay", "vp9dec"),
            VideoCodec::Av1 => ("rtpav1depay", "dav1ddec"),
        };

        let pipeline_str = format!(
            "udpsrc port={} caps=application/x-rtp ! \
             {} ! \
             {} ! \
             videoconvert ! \
             autovideosink sync=false",
            self.stream_port, depay_element, dec_element
        );

        info!("Launching Video Decoder & Display pipeline: gst-launch-1.0 {}", pipeline_str);

        let child = Command::new("gst-launch-1.0")
            .args(pipeline_str.split_whitespace())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn client gst-launch-1.0 pipeline")?;

        self.child = Some(child);
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        if let Some(mut child) = self.child.take() {
            info!("Stopping video decoder pipeline");
            let _ = child.kill();
            let _ = child.wait();
        }
        Ok(())
    }
}

impl Drop for VideoDisplayPipeline {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
