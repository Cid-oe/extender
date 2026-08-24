use anyhow::{Context, Result};
use extender_common::protocol::{
    HandshakeRequest, HandshakeResponse, PacketHeader, PacketType, VideoCodec,
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

        let encoded_req = bincode::serialize(&req)?;
        let header = PacketHeader::new(
            PacketType::HandshakeReq,
            1,
            0,
            encoded_req.len() as u32,
            crc32fast::Hasher::new().finalize(),
        );

        let mut packet = bincode::serialize(&header)?;
        packet.extend(encoded_req);

        socket.send_to(&packet, self.server_addr).await?;
        info!("Sent HandshakeRequest. Waiting for server response...");

        let mut buf = vec![0u8; 65535];
        let (len, _) = tokio::time::timeout(Duration::from_secs(5), socket.recv_from(&mut buf))
            .await
            .context("Handshake timeout: host did not respond")??;

        let resp_header: PacketHeader = bincode::deserialize(&buf[..32.min(len)])?;
        if resp_header.packet_type != PacketType::HandshakeResp {
            anyhow::bail!("Unexpected packet type in handshake response");
        }

        let resp: HandshakeResponse = bincode::deserialize(&buf[32.min(len)..len])?;
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
                let ping_header = PacketHeader::new(PacketType::Ping, seq, 0, 0, 0);
                if let Ok(ping_bytes) = bincode::serialize(&ping_header) {
                    let _ = ping_socket.send_to(&ping_bytes, server_addr).await;
                }
            }
        });

        // Event / keepalive receiver loop
        let mut recv_buf = vec![0u8; 2048];
        loop {
            if let Ok((rlen, _)) = socket_arc.recv_from(&mut recv_buf).await {
                if let Ok(hdr) = bincode::deserialize::<PacketHeader>(&recv_buf[..32.min(rlen)]) {
                    if hdr.packet_type == PacketType::Pong {
                        // Pong received
                    }
                }
            }
        }
    }
}
