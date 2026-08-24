use anyhow::{Context, Result};
use extender_common::protocol::VideoCodec;
use std::process::{Child, Command, Stdio};
use tracing::info;

pub struct VideoEncoderPipeline {
    child: Option<Child>,
    pub codec: VideoCodec,
    pub width: u32,
    pub height: u32,
    pub bitrate_kbps: u32,
}

impl VideoEncoderPipeline {
    pub fn new(codec: VideoCodec, width: u32, height: u32, bitrate_kbps: u32) -> Self {
        Self {
            child: None,
            codec,
            width,
            height,
            bitrate_kbps,
        }
    }

    /// Spawns a low-latency GStreamer pipeline capturing from PipeWire node and streaming RTP over UDP
    pub fn start_stream(&mut self, pipewire_node_id: u32, target_ip: &str, target_port: u16) -> Result<()> {
        let (enc_element, payload_element) = match self.codec {
            VideoCodec::H264Vaapi => (
                format!("videoconvert ! video/x-raw,format=NV12 ! vaapih264enc bitrate={} rate-control=cbr tune=low-latency", self.bitrate_kbps),
                "rtph264pay config-interval=1 pt=96",
            ),
            VideoCodec::H264Nvenc => (
                format!("videoconvert ! video/x-raw,format=NV12 ! nvh264enc bitrate={} preset=low-latency-hq rc-mode=cbr", self.bitrate_kbps),
                "rtph264pay config-interval=1 pt=96",
            ),
            VideoCodec::H264Software => (
                format!("videoconvert ! video/x-raw,format=I420 ! x264enc bitrate={} speed-preset=ultrafast tune=zerolatency bframes=0 key-int-max=30", self.bitrate_kbps),
                "rtph264pay config-interval=1 pt=96",
            ),
            VideoCodec::H265Vaapi => (
                format!("videoconvert ! video/x-raw,format=NV12 ! vaapih265enc bitrate={} rate-control=cbr tune=low-latency", self.bitrate_kbps),
                "rtph265pay config-interval=1 pt=96",
            ),
            VideoCodec::H265Nvenc => (
                format!("videoconvert ! video/x-raw,format=NV12 ! nvh265enc bitrate={} preset=low-latency-hq rc-mode=cbr", self.bitrate_kbps),
                "rtph265pay config-interval=1 pt=96",
            ),
            VideoCodec::Vp8 => (
                format!("videoconvert ! video/x-raw,format=I420 ! vp8enc target-bitrate={} deadline=1 cpu-used=8", self.bitrate_kbps * 1000),
                "rtpvp8pay pt=96",
            ),
            VideoCodec::Vp9 => (
                format!("videoconvert ! video/x-raw,format=I420 ! vp9enc target-bitrate={} deadline=1 cpu-used=8", self.bitrate_kbps * 1000),
                "rtpvp9pay pt=96",
            ),
            VideoCodec::Av1 => (
                format!("videoconvert ! video/x-raw,format=I420 ! rav1eenc speed=10 low-latency=true bitrate={}", self.bitrate_kbps),
                "rtpav1pay pt=96",
            ),
        };

        let node_arg = if pipewire_node_id > 0 {
            format!("path={}", pipewire_node_id)
        } else {
            "client-name=ExtenderHost".to_string()
        };

        let pipeline_str = format!(
            "pipewiresrc {} do-timestamp=true keepalive-time=1000 ! \
             {} ! \
             {} ! \
             udpsink host={} port={} sync=false async=false",
            node_arg, enc_element, payload_element, target_ip, target_port
        );

        info!("Launching Video Encoding pipeline: gst-launch-1.0 {}", pipeline_str);

        let child = Command::new("gst-launch-1.0")
            .args(pipeline_str.split_whitespace())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .context("Failed to spawn gst-launch-1.0 pipeline")?;

        self.child = Some(child);
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        if let Some(mut child) = self.child.take() {
            info!("Stopping video encoding pipeline");
            let _ = child.kill();
            let _ = child.wait();
        }
        Ok(())
    }
}

impl Drop for VideoEncoderPipeline {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
