use anyhow::{Context, Result};
use std::collections::HashMap;
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
    pub async fn create_virtual_monitor(&mut self, width: u32, height: u32) -> Result<u32> {
        info!("Requesting virtual monitor ({}x{}) from GNOME Mutter", width, height);

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

        // Step 2: Create RecordVirtual on the session
        let session_proxy = zbus::Proxy::new(
            &self.connection,
            "org.gnome.Mutter.ScreenCast",
            &session_path,
            "org.gnome.Mutter.ScreenCast.Session",
        )
        .await
        .context("Failed to create ScreenCast.Session proxy")?;

        let mut record_properties: HashMap<&str, Value> = HashMap::new();
        record_properties.insert("width", Value::from(width as i32));
        record_properties.insert("height", Value::from(height as i32));
        record_properties.insert("cursor-mode", Value::from(1u32));

        let stream_path: OwnedObjectPath = session_proxy
            .call("RecordVirtual", &(record_properties))
            .await
            .context("Failed to call org.gnome.Mutter.ScreenCast.Session.RecordVirtual")?;

        info!("Virtual output stream created at: {}", stream_path.as_str());

        // Step 3: Start the Session
        let () = session_proxy
            .call("Start", &())
            .await
            .context("Failed to call Start on ScreenCast session")?;

        // Step 4: Retrieve PipeWire Node ID from Stream properties
        let stream_proxy = zbus::Proxy::new(
            &self.connection,
            "org.gnome.Mutter.ScreenCast",
            &stream_path,
            "org.gnome.Mutter.ScreenCast.Stream",
        )
        .await
        .context("Failed to create ScreenCast.Stream proxy")?;

        let pipewire_node_id: u32 = stream_proxy
            .get_property("PipeWireNodeId")
            .await
            .unwrap_or(0);

        info!("Allocated PipeWire Node ID for virtual monitor: {}", pipewire_node_id);

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
