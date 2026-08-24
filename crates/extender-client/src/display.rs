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
        let (caps_str, depay_and_decode) = match self.codec {
            VideoCodec::H264Vaapi => (
                "application/x-rtp,media=video,clock-rate=90000,encoding-name=H264,payload=96",
                "rtph264depay ! h264parse ! vaapih264dec ! videoconvert ! autovideosink sync=false",
            ),
            VideoCodec::H264Nvenc => (
                "application/x-rtp,media=video,clock-rate=90000,encoding-name=H264,payload=96",
                "rtph264depay ! h264parse ! nvh264dec ! videoconvert ! autovideosink sync=false",
            ),
            VideoCodec::H264Software => (
                "application/x-rtp,media=video,clock-rate=90000,encoding-name=H264,payload=96",
                "rtph264depay ! h264parse ! avdec_h264 ! videoconvert ! autovideosink sync=false",
            ),
            VideoCodec::H265Vaapi => (
                "application/x-rtp,media=video,clock-rate=90000,encoding-name=H265,payload=96",
                "rtph265depay ! h265parse ! vaapih265dec ! videoconvert ! autovideosink sync=false",
            ),
            VideoCodec::H265Nvenc => (
                "application/x-rtp,media=video,clock-rate=90000,encoding-name=H265,payload=96",
                "rtph265depay ! h265parse ! nvh265dec ! videoconvert ! autovideosink sync=false",
            ),
            VideoCodec::Vp8 => (
                "application/x-rtp,media=video,clock-rate=90000,encoding-name=VP8,payload=96",
                "rtpvp8depay ! vp8dec ! videoconvert ! autovideosink sync=false",
            ),
            VideoCodec::Vp9 => (
                "application/x-rtp,media=video,clock-rate=90000,encoding-name=VP9,payload=96",
                "rtpvp9depay ! vp9dec ! videoconvert ! autovideosink sync=false",
            ),
            VideoCodec::Av1 => (
                "application/x-rtp,media=video,clock-rate=90000,encoding-name=AV1,payload=96",
                "rtpav1depay ! dav1ddec ! videoconvert ! autovideosink sync=false",
            ),
        };

        let pipeline_str = format!(
            "udpsrc port={} caps={} ! {}",
            self.stream_port, caps_str, depay_and_decode
        );

        info!("Launching Video Decoder & Display pipeline: gst-launch-1.0 {}", pipeline_str);

        let child = Command::new("gst-launch-1.0")
            .args(pipeline_str.split_whitespace())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
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
