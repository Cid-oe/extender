use anyhow::{Context, Result};
use extender_common::protocol::{
    HandshakeResponse, Packet, PacketPayload, VideoCodec,
};
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

use crate::capture::VideoEncoderPipeline;
use crate::input_sink::InputInjector;
use crate::mutter::MutterVirtualMonitorManager;

pub struct ExtenderServer {
    mutter: Arc<Mutex<MutterVirtualMonitorManager>>,
    input_injector: Arc<Mutex<InputInjector>>,
    stream_port: u16,
    input_port: u16,
    pub width: u32,
    pub height: u32,
    codec: VideoCodec,
    bitrate_kbps: u32,
}

impl ExtenderServer {
    pub async fn new(
        width: u32,
        height: u32,
        stream_port: u16,
        input_port: u16,
        codec: VideoCodec,
        bitrate_kbps: u32,
    ) -> Result<Self> {
        let mutter = MutterVirtualMonitorManager::new()
            .await
            .context("Failed to initialize Mutter D-Bus manager")?;
        let input_injector = InputInjector::new(width, height)
            .context("Failed to initialize Input Injector")?;

        Ok(Self {
            mutter: Arc::new(Mutex::new(mutter)),
            input_injector: Arc::new(Mutex::new(input_injector)),
            stream_port,
            input_port,
            width,
            height,
            codec,
            bitrate_kbps,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        info!("Starting Extender Server listening on input port {}", self.input_port);

        let input_socket = UdpSocket::bind(format!("0.0.0.0:{}", self.input_port))
            .await
            .context("Failed to bind input UDP socket")?;

        let mut buf = vec![0u8; 65535];
        let mut encoder_pipeline: Option<VideoEncoderPipeline> = None;

        loop {
            let (len, client_addr) = match input_socket.recv_from(&mut buf).await {
                Ok(res) => res,
                Err(e) => {
                    error!("Error receiving packet on input socket: {}", e);
                    continue;
                }
            };

            let packet = match Packet::decode(&buf[..len]) {
                Ok(p) => p,
                Err(e) => {
                    warn!("Received invalid packet from {} (len: {}): {}", client_addr, len, e);
                    continue;
                }
            };

            match packet.payload {
                PacketPayload::HandshakeReq(req) => {
                    info!(
                        "Received HandshakeRequest from client {} (name: {}, preferred: {}x{}@{}Hz)",
                        client_addr, req.client_name, req.preferred_width, req.preferred_height, req.refresh_rate
                    );

                    let (target_w, target_h) = (req.preferred_width, req.preferred_height);
                    let selected_codec = if req.supported_codecs.contains(&self.codec) {
                        self.codec
                    } else {
                        req.supported_codecs.first().copied().unwrap_or(VideoCodec::H264Software)
                    };

                    info!("Allocating Mutter virtual monitor: {}x{}", target_w, target_h);
                    let mut mutter_guard = self.mutter.lock().await;
                    let pw_node_id = mutter_guard
                        .create_virtual_monitor(target_w, target_h)
                        .await
                        .unwrap_or_else(|e| {
                            warn!("Virtual monitor creation failed or running without active Mutter: {}", e);
                            0
                        });

                    let mut pipeline = VideoEncoderPipeline::new(
                        selected_codec,
                        target_w,
                        target_h,
                        self.bitrate_kbps,
                    );

                    let client_ip = client_addr.ip().to_string();
                    info!("Starting video encoding pipeline towards client {}:{}", client_ip, self.stream_port);
                    if let Err(e) = pipeline.start_stream(pw_node_id, &client_ip, self.stream_port) {
                        error!("Failed to start encoder pipeline: {}", e);
                    }
                    encoder_pipeline = Some(pipeline);

                    let response = HandshakeResponse {
                        accepted: true,
                        error_message: None,
                        selected_width: target_w,
                        selected_height: target_h,
                        selected_codec,
                        pipewire_node_id: Some(pw_node_id),
                        stream_port: self.stream_port,
                        input_port: self.input_port,
                    };

                    let resp_packet = Packet::new(PacketPayload::HandshakeResp(response));
                    if let Ok(encoded_resp) = resp_packet.encode() {
                        let _ = input_socket.send_to(&encoded_resp, client_addr).await;
                        info!("Sent HandshakeResponse to {}", client_addr);
                    }
                }
                PacketPayload::InputData(event) => {
                    let mut injector = self.input_injector.lock().await;
                    let _ = injector.inject_event(event);
                }
                PacketPayload::Ping { sequence, timestamp_us } => {
                    let pong = Packet::new(PacketPayload::Pong { sequence, timestamp_us });
                    if let Ok(pong_bytes) = pong.encode() {
                        let _ = input_socket.send_to(&pong_bytes, client_addr).await;
                    }
                }
                PacketPayload::Disconnect => {
                    info!("Client {} requested disconnect", client_addr);
                    if let Some(mut pipeline) = encoder_pipeline.take() {
                        let _ = pipeline.stop();
                    }
                    let mut mutter_guard = self.mutter.lock().await;
                    let _ = mutter_guard.stop().await;
                }
                _ => {}
            }
        }
    }
}
