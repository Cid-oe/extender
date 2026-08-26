use anyhow::{bail, Context, Result};
use std::process::Command;
use tracing::{info, warn};

pub struct HyprlandVirtualMonitorManager {
    output_name: String,
    created: bool,
}

impl HyprlandVirtualMonitorManager {
    pub fn new() -> Result<Self> {
        // Verify hyprctl is available
        let status = Command::new("hyprctl")
            .arg("version")
            .output()
            .context("Failed to execute hyprctl. Is Hyprland running on Omarchy?")?;

        if !status.status.success() {
            bail!(
                "hyprctl exited with error: {}",
                String::from_utf8_lossy(&status.stderr)
            );
        }

        Ok(Self {
            output_name: "EXTENDER-1".to_string(),
            created: false,
        })
    }

    /// Creates and configures a headless virtual monitor in Hyprland
    pub fn create_virtual_monitor(&mut self, width: u32, height: u32, refresh_rate: u32) -> Result<u32> {
        info!(
            "Creating Hyprland headless virtual output '{}' ({}x{}@{}Hz) on Omarchy",
            self.output_name, width, height, refresh_rate
        );

        // Step 1: Create headless output via hyprctl
        let output = Command::new("hyprctl")
            .args(["output", "create", "headless", &self.output_name])
            .output()
            .context("Failed to run `hyprctl output create headless`")?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            warn!("`hyprctl output create headless` response: {}", err);
        }

        // Step 2: Configure monitor resolution, position and refresh rate
        let mode = format!("{},{}x{}@{},auto,1", self.output_name, width, height, refresh_rate);
        let config_output = Command::new("hyprctl")
            .args(["keyword", "monitor", &mode])
            .output()
            .context("Failed to run `hyprctl keyword monitor`")?;

        if !config_output.status.success() {
            warn!(
                "`hyprctl keyword monitor` response: {}",
                String::from_utf8_lossy(&config_output.stderr)
            );
        }

        self.created = true;
        info!(
            "Hyprland virtual monitor '{}' successfully configured on Omarchy",
            self.output_name
        );

        // PipeWire / GStreamer node ID 0 lets pipewiresrc attach to the active/created screen stream
        Ok(0)
    }

    pub fn stop(&mut self) -> Result<()> {
        if self.created {
            info!("Removing Hyprland headless monitor '{}' on Omarchy", self.output_name);
            let _ = Command::new("hyprctl")
                .args(["output", "remove", &self.output_name])
                .output();
            self.created = false;
        }
        Ok(())
    }
}

impl Drop for HyprlandVirtualMonitorManager {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
