use anyhow::Result;
use extender_common::events::InputEvent;
use extender_common::protocol::{Packet, PacketPayload};
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tracing::debug;

pub struct InputSender {
    socket: UdpSocket,
    server_addr: SocketAddr,
}

impl InputSender {
    pub async fn new(server_addr: SocketAddr) -> Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        Ok(Self {
            socket,
            server_addr,
        })
    }

    pub async fn send_event(&mut self, event: InputEvent) -> Result<()> {
        let packet = Packet::new(PacketPayload::InputData(event));
        let packet_bytes = packet.encode()?;
        self.socket.send_to(&packet_bytes, self.server_addr).await?;
        debug!("Sent input event packet");
        Ok(())
    }
}
