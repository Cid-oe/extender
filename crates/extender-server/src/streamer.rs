use anyhow::{Context, Result};
use extender_common::events::InputEvent;
use extender_common::protocol::{
    HandshakeRequest, HandshakeResponse, PacketHeader, PacketType, VideoCodec,
};
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
use tracing::{error, info};

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

            if len < std::mem::size_of::<PacketHeader>() {
                continue;
            }

            if let Ok(header) = bincode::deserialize::<PacketHeader>(&buf[..32.min(len)]) {
                if !header.validate() {
                    continue;
                }

                match header.packet_type {
                    PacketType::HandshakeReq => {
                        info!("Received HandshakeRequest from client {}", client_addr);
                        let payload = &buf[32.min(len)..len];
                        if let Ok(req) = bincode::deserialize::<HandshakeRequest>(payload) {
                            let (target_w, target_h) = (req.preferred_width, req.preferred_height);
                            let selected_codec = if req.supported_codecs.contains(&self.codec) {
                                self.codec
                            } else {
                                req.supported_codecs.first().copied().unwrap_or(VideoCodec::H264Software)
                            };

                            let mut mutter_guard = self.mutter.lock().await;
                            let pw_node_id = mutter_guard.create_virtual_monitor(target_w, target_h).await.unwrap_or(0);

                            let mut pipeline = VideoEncoderPipeline::new(
                                selected_codec,
                                target_w,
                                target_h,
                                self.bitrate_kbps,
                            );

                            let client_ip = client_addr.ip().to_string();
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

                            let encoded_resp = bincode::serialize(&response)?;
                            let resp_header = PacketHeader::new(
                                PacketType::HandshakeResp,
                                1,
                                0,
                                encoded_resp.len() as u32,
                                crc32fast::Hasher::new().finalize(),
                            );
                            let mut resp_packet = bincode::serialize(&resp_header)?;
                            resp_packet.extend(encoded_resp);

                            let _ = input_socket.send_to(&resp_packet, client_addr).await;
                        }
                    }
                    PacketType::InputData => {
                        let payload = &buf[32.min(len)..len];
                        if let Ok(event) = bincode::deserialize::<InputEvent>(payload) {
                            let mut injector = self.input_injector.lock().await;
                            let _ = injector.inject_event(event);
                        }
                    }
                    PacketType::Ping => {
                        let pong_header = PacketHeader::new(
                            PacketType::Pong,
                            header.sequence,
                            header.timestamp_us,
                            0,
                            0,
                        );
                        if let Ok(pong_bytes) = bincode::serialize(&pong_header) {
                            let _ = input_socket.send_to(&pong_bytes, client_addr).await;
                        }
                    }
                    PacketType::Disconnect => {
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
}
