use anyhow::{Context, Result};
use extender_common::protocol::{
    HandshakeRequest, Packet, PacketPayload, VideoCodec,
};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UdpSocket;
use tracing::info;

use crate::display::VideoDisplayPipeline;

pub struct ExtenderClient {
    server_addr: SocketAddr,
    preferred_width: u32,
    preferred_height: u32,
    refresh_rate: u32,
    supported_codecs: Vec<VideoCodec>,
}

impl ExtenderClient {
    pub fn new(
        server_addr: SocketAddr,
        preferred_width: u32,
        preferred_height: u32,
        refresh_rate: u32,
        supported_codecs: Vec<VideoCodec>,
    ) -> Self {
        Self {
            server_addr,
            preferred_width,
            preferred_height,
            refresh_rate,
            supported_codecs,
        }
    }

    pub async fn connect_and_run(&self) -> Result<()> {
        info!("Connecting to Extender host at {}", self.server_addr);

        let socket = UdpSocket::bind("0.0.0.0:0")
            .await
            .context("Failed to bind client UDP socket")?;

        // Perform Handshake
        let req = HandshakeRequest {
            client_name: "ExtenderClient-Wayland".to_string(),
            preferred_width: self.preferred_width,
            preferred_height: self.preferred_height,
            refresh_rate: self.refresh_rate,
            supported_codecs: self.supported_codecs.clone(),
            auth_token: None,
        };

        let req_packet = Packet::new(PacketPayload::HandshakeReq(req));
        let packet_bytes = req_packet.encode()?;

        socket.send_to(&packet_bytes, self.server_addr).await?;
        info!("Sent HandshakeRequest to {}. Waiting for server response...", self.server_addr);

        let mut buf = vec![0u8; 65535];
        let (len, from_addr) = tokio::time::timeout(Duration::from_secs(5), socket.recv_from(&mut buf))
            .await
            .context("Handshake timeout: host did not respond within 5s")??;

        info!("Received response packet ({} bytes) from {}", len, from_addr);
        let (resp_packet, _is_legacy) = Packet::decode(&buf[..len])?;

        let resp = match resp_packet.payload {
            PacketPayload::HandshakeResp(r) => r,
            other => anyhow::bail!("Unexpected packet type in handshake response: {:?}", other),
        };

        if !resp.accepted {
            anyhow::bail!("Host rejected connection: {:?}", resp.error_message);
        }

        info!(
            "Connected! Allocated virtual screen: {}x{}, Codec: {:?}, Stream Port: {}",
            resp.selected_width, resp.selected_height, resp.selected_codec, resp.stream_port
        );

        // Start Display Pipeline
        let mut display = VideoDisplayPipeline::new(resp.selected_codec, resp.stream_port);
        display.start_display()?;

        // Keepalive / Ping loop
        let socket_arc = Arc::new(socket);
        let server_addr = self.server_addr;

        let ping_socket = socket_arc.clone();
        tokio::spawn(async move {
            let mut seq = 100u64;
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                seq += 1;
                let ping = Packet::new(PacketPayload::Ping { sequence: seq, timestamp_us: 0 });
                if let Ok(ping_bytes) = ping.encode() {
                    let _ = ping_socket.send_to(&ping_bytes, server_addr).await;
                }
            }
        });

        // Event / keepalive receiver loop
        let mut recv_buf = vec![0u8; 2048];
        loop {
            if let Ok((rlen, _)) = socket_arc.recv_from(&mut recv_buf).await {
                if let Ok((pkt, _)) = Packet::decode(&recv_buf[..rlen]) {
                    if let PacketPayload::Pong { .. } = pkt.payload {
                        // Pong received
                    }
                }
            }
        }
    }
}
