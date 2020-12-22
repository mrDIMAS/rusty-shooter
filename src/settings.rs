use crate::assets;
use rg3d::{sound::context, sound::context::Context, utils::log::Log};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoundSettings {
    pub sound_volume: f32,
    pub hrtf: bool,
}

impl Default for SoundSettings {
    fn default() -> Self {
        Self {
            sound_volume: 1.0,
            hrtf: true,
        }
    }
}

impl SoundSettings {
    pub fn is_hrtf(sound_context: &Context) -> bool {
        if let rg3d::sound::renderer::Renderer::HrtfRenderer(_) = sound_context.renderer() {
            true
        } else {
            false
        }
    }

    pub fn hrtf_on(sound_context: &mut Context) {
        let hrtf_sphere = rg3d::sound::hrtf::HrirSphere::from_file(
            assets::sounds::HRTF_HRIR,
            context::SAMPLE_RATE,
        )
        .unwrap();
        sound_context.set_renderer(rg3d::sound::renderer::Renderer::HrtfRenderer(
            rg3d::sound::renderer::hrtf::HrtfRenderer::new(hrtf_sphere),
        ));
    }

    pub fn hrtf_off(sound_context: &mut Context) {
        sound_context.set_renderer(rg3d::sound::renderer::Renderer::Default);
    }

    pub fn get_from_engine(sound_context: &Context) -> Self {
        Self {
            sound_volume: sound_context.master_gain(),
            hrtf: Self::is_hrtf(sound_context),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub renderer: rg3d::renderer::QualitySettings,
    #[serde(default)]
    pub controls: crate::control_scheme::ControlScheme,
    #[serde(default)]
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
