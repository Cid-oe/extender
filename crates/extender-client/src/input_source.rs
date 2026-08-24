use anyhow::Result;
use extender_common::events::InputEvent;
use extender_common::protocol::{PacketHeader, PacketType};
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tracing::debug;

pub struct InputSender {
    socket: UdpSocket,
    server_addr: SocketAddr,
    sequence: u64,
}

impl InputSender {
    pub async fn new(server_addr: SocketAddr) -> Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        Ok(Self {
            socket,
            server_addr,
            sequence: 0,
        })
    }

    pub async fn send_event(&mut self, event: InputEvent) -> Result<()> {
        self.sequence += 1;
        let payload = bincode::serialize(&event)?;
        let header = PacketHeader::new(
            PacketType::InputData,
            self.sequence,
            0,
            payload.len() as u32,
            crc32fast::Hasher::new().finalize(),
        );

        let mut packet = bincode::serialize(&header)?;
        packet.extend(payload);

        self.socket.send_to(&packet, self.server_addr).await?;
        debug!("Sent input event seq {}", self.sequence);
        Ok(())
    }
}
