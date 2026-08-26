pub mod capture;
pub mod hyprland;
pub mod input_sink;
pub mod monitor;
pub mod mutter;
pub mod streamer;

pub use capture::VideoEncoderPipeline;
pub use hyprland::HyprlandVirtualMonitorManager;
pub use input_sink::InputInjector;
pub use monitor::{CompositorBackend, VirtualMonitorBackend, VirtualMonitorManager};
pub use mutter::MutterVirtualMonitorManager;
pub use streamer::ExtenderServer;

