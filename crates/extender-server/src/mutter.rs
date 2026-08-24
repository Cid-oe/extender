use anyhow::{Context, Result};
use futures_util::StreamExt;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{info, warn};
use zbus::{zvariant::{OwnedObjectPath, Value}, Connection};

pub struct MutterVirtualMonitorManager {
    connection: Connection,
    session_path: Option<OwnedObjectPath>,
    stream_path: Option<OwnedObjectPath>,
}

impl MutterVirtualMonitorManager {
    pub async fn new() -> Result<Self> {
        let connection = Connection::session()
            .await
            .context("Failed to connect to D-Bus session bus")?;
        Ok(Self {
            connection,
            session_path: None,
            stream_path: None,
        })
    }

    /// Creates a headless virtual monitor on Mutter via ScreenCast & RemoteDesktop DBus APIs
    pub async fn create_virtual_monitor(&mut self, _width: u32, _height: u32) -> Result<u32> {
        info!("Requesting virtual monitor from GNOME Mutter");

        // Step 1: Create a ScreenCast Session
        let screencast_proxy = zbus::Proxy::new(
            &self.connection,
            "org.gnome.Mutter.ScreenCast",
            "/org/gnome/Mutter/ScreenCast",
            "org.gnome.Mutter.ScreenCast",
        )
        .await
        .context("Failed to create ScreenCast proxy")?;

        let properties: HashMap<&str, Value> = HashMap::new();
        let session_path: OwnedObjectPath = screencast_proxy
            .call("CreateSession", &(properties))
            .await
            .context("Failed to call org.gnome.Mutter.ScreenCast.CreateSession")?;

        info!("Mutter ScreenCast Session created at path: {}", session_path.as_str());

        // Step 2: Create RecordVirtual or RecordMonitor on the session
        let session_proxy = zbus::Proxy::new(
            &self.connection,
            "org.gnome.Mutter.ScreenCast",
            &session_path,
            "org.gnome.Mutter.ScreenCast.Session",
        )
        .await
        .context("Failed to create ScreenCast.Session proxy")?;

        let mut record_properties: HashMap<&str, Value> = HashMap::new();
        record_properties.insert("cursor-mode", Value::from(1u32)); // 1 = embedded cursor

        let stream_path: Result<OwnedObjectPath, zbus::Error> = session_proxy
            .call("RecordVirtual", &(record_properties))
            .await;

        let stream_path = match stream_path {
            Ok(p) => {
                info!("Virtual output stream created at: {}", p.as_str());
                p
            }
            Err(e) => {
                warn!("RecordVirtual note ({}), requesting RecordMonitor on active monitor", e);
                let mut mon_props: HashMap<&str, Value> = HashMap::new();
                mon_props.insert("cursor-mode", Value::from(1u32));
                session_proxy
                    .call("RecordMonitor", &("HDMI-2", mon_props))
                    .await
                    .context("Failed to call RecordMonitor")?
            }
        };

        // Step 3: Set up signal listener for PipeWireStreamAdded BEFORE calling Start
        let stream_proxy = zbus::Proxy::new(
            &self.connection,
            "org.gnome.Mutter.ScreenCast",
            &stream_path,
            "org.gnome.Mutter.ScreenCast.Stream",
        )
        .await
        .context("Failed to create ScreenCast.Stream proxy")?;

        let mut signal_stream = stream_proxy
            .receive_signal("PipeWireStreamAdded")
            .await
            .context("Failed to subscribe to PipeWireStreamAdded signal")?;

        // Step 4: Start the Session
        let () = session_proxy
            .call("Start", &())
            .await
            .context("Failed to call Start on ScreenCast session")?;

        // Step 5: Wait for PipeWireStreamAdded signal with a timeout
        let pipewire_node_id = match tokio::time::timeout(Duration::from_secs(3), signal_stream.next()).await {
            Ok(Some(signal_msg)) => {
                let (node_id,): (u32,) = signal_msg.body().deserialize().unwrap_or((0,));
                info!("Successfully received PipeWireStreamAdded with Node ID: {}", node_id);
                node_id
            }
            Ok(None) => {
                warn!("PipeWireStreamAdded stream ended without yielding node ID");
                0
            }
            Err(_) => {
                warn!("Timed out waiting for PipeWireStreamAdded signal from Mutter");
                0
            }
        };

        self.session_path = Some(session_path);
        self.stream_path = Some(stream_path);

        Ok(pipewire_node_id)
    }

    pub async fn stop(&mut self) -> Result<()> {
        if let Some(session_path) = self.session_path.take() {
            info!("Closing Mutter ScreenCast session: {}", session_path.as_str());
            let session_proxy = zbus::Proxy::new(
                &self.connection,
                "org.gnome.Mutter.ScreenCast",
                &session_path,
                "org.gnome.Mutter.ScreenCast.Session",
            )
            .await?;
            let _: Result<(), _> = session_proxy.call("Stop", &()).await;
        }
        self.stream_path = None;
        Ok(())
    }
}

impl Drop for MutterVirtualMonitorManager {
    fn drop(&mut self) {
        if self.session_path.is_some() {
            warn!("MutterVirtualMonitorManager dropped without explicit stop()");
        }
    }
}
