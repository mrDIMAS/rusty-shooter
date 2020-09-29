use rg3d::utils::log::Log;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoundSettings {
    pub sound_volume: f32,
}

impl Default for SoundSettings {
    fn default() -> Self {
        Self { sound_volume: 1.0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub renderer: rg3d::renderer::QualitySettings,
    pub controls: crate::control_scheme::ControlScheme,
    pub sound: SoundSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            renderer: rg3d::renderer::QualitySettings::default(),
            controls: crate::control_scheme::ControlScheme::default(),
            sound: SoundSettings::default(),
        }
    }
}

impl Settings {
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
}
