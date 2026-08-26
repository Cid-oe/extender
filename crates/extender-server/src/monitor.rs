use anyhow::Result;
use clap::ValueEnum;
use std::env;
use tracing::{info, warn};

use crate::hyprland::HyprlandVirtualMonitorManager;
use crate::mutter::MutterVirtualMonitorManager;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum CompositorBackend {
    #[default]
    Auto,
    Hyprland,
    Mutter,
    Headless,
}

pub enum VirtualMonitorBackend {
    Hyprland(HyprlandVirtualMonitorManager),
    Mutter(MutterVirtualMonitorManager),
    Headless,
}

pub struct VirtualMonitorManager {
    backend: VirtualMonitorBackend,
}

impl VirtualMonitorManager {
    pub async fn new(backend_choice: CompositorBackend) -> Result<Self> {
        let backend = match backend_choice {
            CompositorBackend::Hyprland => {
                info!("Using Hyprland / Omarchy backend for virtual monitor management");
                VirtualMonitorBackend::Hyprland(HyprlandVirtualMonitorManager::new()?)
            }
            CompositorBackend::Mutter => {
                info!("Using GNOME Mutter D-Bus backend for virtual monitor management");
                VirtualMonitorBackend::Mutter(MutterVirtualMonitorManager::new().await?)
            }
            CompositorBackend::Headless => {
                info!("Using Headless / Direct PipeWire backend (no compositor monitor creation)");
                VirtualMonitorBackend::Headless
            }
            CompositorBackend::Auto => {
                // Check Omarchy / Hyprland indicators
                let is_hyprland = env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok()
                    || env::var("XDG_CURRENT_DESKTOP")
                        .map(|v| {
                            let v_lower = v.to_lowercase();
                            v_lower.contains("hyprland") || v_lower.contains("omarchy")
                        })
                        .unwrap_or(false);

                if is_hyprland {
                    match HyprlandVirtualMonitorManager::new() {
                        Ok(mgr) => {
                            info!("Auto-detected Omarchy / Hyprland compositor session");
                            VirtualMonitorBackend::Hyprland(mgr)
                        }
                        Err(e) => {
                            warn!(
                                "Failed to initialize Hyprland backend ({}), falling back to Headless",
                                e
                            );
                            VirtualMonitorBackend::Headless
                        }
                    }
                } else {
                    // Try Mutter (GNOME)
                    match MutterVirtualMonitorManager::new().await {
                        Ok(mgr) => {
                            info!("Auto-detected GNOME Mutter session");
                            VirtualMonitorBackend::Mutter(mgr)
                        }
                        Err(e) => {
                            // If Mutter is unavailable, check if hyprctl is present
                            if let Ok(mgr) = HyprlandVirtualMonitorManager::new() {
                                info!("Found hyprctl on system, using Hyprland / Omarchy backend");
                                VirtualMonitorBackend::Hyprland(mgr)
                            } else {
                                warn!(
                                    "Mutter D-Bus not available ({}), falling back to Headless PipeWire backend",
                                    e
                                );
                                VirtualMonitorBackend::Headless
                            }
                        }
                    }
                }
            }
        };

        Ok(Self { backend })
    }

    pub async fn create_virtual_monitor(
        &mut self,
        width: u32,
        height: u32,
        refresh_rate: u32,
    ) -> Result<u32> {
        match &mut self.backend {
            VirtualMonitorBackend::Hyprland(mgr) => {
                mgr.create_virtual_monitor(width, height, refresh_rate)
            }
            VirtualMonitorBackend::Mutter(mgr) => mgr.create_virtual_monitor(width, height).await,
            VirtualMonitorBackend::Headless => {
                info!("Headless mode: Ready to capture active PipeWire stream");
                Ok(0)
            }
        }
    }

    pub async fn stop(&mut self) -> Result<()> {
        match &mut self.backend {
            VirtualMonitorBackend::Hyprland(mgr) => mgr.stop(),
            VirtualMonitorBackend::Mutter(mgr) => mgr.stop().await,
            VirtualMonitorBackend::Headless => Ok(()),
        }
    }
}
