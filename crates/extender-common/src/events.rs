use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyAction {
    Press,
    Release,
    Repeat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    Back,
    Forward,
    Other(u8),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MouseEvent {
    MotionAbsolute { x: f64, y: f64, width: u32, height: u32 },
    MotionRelative { dx: f64, dy: f64 },
    Button { button: MouseButton, state: KeyAction },
    AxisScroll { horizontal: f64, vertical: f64 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InputEvent {
    Mouse(MouseEvent),
    Keyboard {
        keycode: u32,
        action: KeyAction,
        modifiers: u32,
    },
    ClipboardText(String),
    KeepAlive,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_event_serialization() {
        let event = InputEvent::Mouse(MouseEvent::MotionAbsolute {
            x: 500.0,
            y: 300.0,
            width: 1920,
            height: 1080,
        });

        let encoded = bincode::serialize(&event).unwrap();
        let decoded: InputEvent = bincode::deserialize(&encoded).unwrap();
        assert_eq!(event, decoded);
    }
}
