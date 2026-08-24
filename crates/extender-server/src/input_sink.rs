use anyhow::Result;
use extender_common::events::{InputEvent, MouseEvent};
use std::fs::{File, OpenOptions};
use std::path::Path;
use tracing::{debug, info, warn};

pub struct InputInjector {
    _uinput_file: Option<File>,
    virtual_screen_width: u32,
    virtual_screen_height: u32,
}

impl InputInjector {
    pub fn new(width: u32, height: u32) -> Result<Self> {
        let uinput_path = Path::new("/dev/uinput");
        let uinput_file = if uinput_path.exists() {
            match OpenOptions::new().write(true).open(uinput_path) {
                Ok(file) => {
                    info!("Successfully opened /dev/uinput for virtual event injection");
                    Some(file)
                }
                Err(e) => {
                    warn!("Could not open /dev/uinput (permission denied or not root: {}). Running in simulation/log mode.", e);
                    None
                }
            }
        } else {
            warn!("/dev/uinput does not exist. Running input injector in log mode.");
            None
        };

        Ok(Self {
            _uinput_file: uinput_file,
            virtual_screen_width: width,
            virtual_screen_height: height,
        })
    }

    pub fn inject_event(&mut self, event: InputEvent) -> Result<()> {
        match event {
            InputEvent::Mouse(mouse_event) => match mouse_event {
                MouseEvent::MotionAbsolute { x, y, width, height } => {
                    let scaled_x = (x / width as f64) * self.virtual_screen_width as f64;
                    let scaled_y = (y / height as f64) * self.virtual_screen_height as f64;
                    debug!("Injecting Mouse Move Absolute: ({:.1}, {:.1})", scaled_x, scaled_y);
                }
                MouseEvent::MotionRelative { dx, dy } => {
                    debug!("Injecting Mouse Move Relative: ({:.1}, {:.1})", dx, dy);
                }
                MouseEvent::Button { button, state } => {
                    debug!("Injecting Mouse Button {:?}: {:?}", button, state);
                }
                MouseEvent::AxisScroll { horizontal, vertical } => {
                    debug!("Injecting Mouse Scroll: h={:.1}, v={:.1}", horizontal, vertical);
                }
            },
            InputEvent::Keyboard { keycode, action, modifiers } => {
                debug!("Injecting Keyboard Key {}: {:?} (mods: {:#x})", keycode, action, modifiers);
            }
            InputEvent::ClipboardText(text) => {
                info!("Received clipboard sync from client ({} bytes)", text.len());
            }
            InputEvent::KeepAlive => {}
        }
        Ok(())
    }
}
