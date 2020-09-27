use rg3d::event::VirtualKeyCode;
use rg3d::utils::log::Log;
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ControlButton {
    Mouse(u8),
    Key(VirtualKeyCode),
    WheelUp,
    WheelDown,
}

impl ControlButton {
    pub fn name(self) -> &'static str {
        match self {
            ControlButton::Mouse(index) => match index {
                1 => "LMB",
                2 => "RMB",
                3 => "MMB",
                4 => "MB4",
                5 => "MB5",
                _ => "Unknown",
            },
            ControlButton::Key(code) => rg3d::utils::virtual_key_code_name(code),
            ControlButton::WheelUp => "Wheel Up",
            ControlButton::WheelDown => "Wheel Down",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ControlButtonDefinition {
    pub description: String,
    pub button: ControlButton,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ControlScheme {
    pub move_forward: ControlButtonDefinition,
    pub move_backward: ControlButtonDefinition,
    pub move_left: ControlButtonDefinition,
    pub move_right: ControlButtonDefinition,
    pub jump: ControlButtonDefinition,
    pub crouch: ControlButtonDefinition,
    pub ads: ControlButtonDefinition,
    pub shoot: ControlButtonDefinition,
    pub next_weapon: ControlButtonDefinition,
    pub prev_weapon: ControlButtonDefinition,
    pub run: ControlButtonDefinition,
    pub mouse_sens: f32,
    pub mouse_y_inverse: bool,
    pub smooth_mouse: bool,
    pub shake_camera: bool,
}

impl Default for ControlScheme {
    fn default() -> Self {
        Self {
            move_forward: ControlButtonDefinition {
                description: "Move Forward".to_string(),
                button: ControlButton::Key(VirtualKeyCode::W),
            },
            move_backward: ControlButtonDefinition {
                description: "Move Backward".to_string(),
                button: ControlButton::Key(VirtualKeyCode::S),
            },
            move_left: ControlButtonDefinition {
                description: "Move Left".to_string(),
                button: ControlButton::Key(VirtualKeyCode::A),
            },
            move_right: ControlButtonDefinition {
                description: "Move Right".to_string(),
                button: ControlButton::Key(VirtualKeyCode::D),
            },
            jump: ControlButtonDefinition {
                description: "Jump".to_string(),
                button: ControlButton::Key(VirtualKeyCode::Space),
            },
            crouch: ControlButtonDefinition {
                description: "Crouch".to_string(),
                button: ControlButton::Key(VirtualKeyCode::C),
            },
            ads: ControlButtonDefinition {
                description: "Aim Down Sights".to_string(),
                button: ControlButton::Mouse(3),
            },
            shoot: ControlButtonDefinition {
                description: "Shoot".to_string(),
                button: ControlButton::Mouse(1),
            },
            next_weapon: ControlButtonDefinition {
                description: "Next Weapon".to_string(),
                button: ControlButton::WheelUp,
            },
            prev_weapon: ControlButtonDefinition {
                description: "Previous Weapon".to_string(),
                button: ControlButton::WheelDown,
            },
            run: ControlButtonDefinition {
                description: "Run".to_string(),
                button: ControlButton::Key(VirtualKeyCode::LShift),
            },
            mouse_sens: 0.2,
            mouse_y_inverse: false,
            smooth_mouse: true,
            shake_camera: true,
        }
    }
}

impl ControlScheme {
    pub fn load_from_file(filename: &str) -> Self {
        if let Ok(Ok(settings)) = std::fs::read_to_string(std::path::Path::new(filename))
            .as_ref()
            .and_then(|f| serde::export::Ok(serde_json::from_str(f)))
        {
            Log::writeln("Successfully loaded settings".to_string());
            settings
        } else {
            // Unable to read settings file, so fall back to defaults
            Log::writeln(format!(
                "Could not read settings file {} (missing or corrupted?), falling back to defaults",
                filename
            ));
            Self::default()
        }
    }

    pub fn write_to_file(&self, filename: &str) {
        if let Err(error) = serde_json::to_string(self).and_then(|data| {
            serde::export::Ok(std::fs::write(std::path::Path::new(filename), data))
        }) {
            Log::writeln(format!("Error saving settings: {}", error))
        } else {
            Log::writeln(format!("Succesfully saved settings to {}", filename));
        }
    }

    pub fn buttons_mut(&mut self) -> [&mut ControlButtonDefinition; 11] {
        [
            &mut self.move_forward,
            &mut self.move_backward,
            &mut self.move_left,
            &mut self.move_right,
            &mut self.jump,
            &mut self.crouch,
            &mut self.ads,
            &mut self.shoot,
            &mut self.next_weapon,
            &mut self.prev_weapon,
            &mut self.run,
        ]
    }

    pub fn buttons(&self) -> [&ControlButtonDefinition; 11] {
        [
            &self.move_forward,
            &self.move_backward,
            &self.move_left,
            &self.move_right,
            &self.jump,
            &self.crouch,
            &self.ads,
            &self.shoot,
            &self.next_weapon,
            &self.prev_weapon,
            &self.run,
        ]
    }

    pub fn reset(&mut self) {
        *self = Default::default();
    }
}
