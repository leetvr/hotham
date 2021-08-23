use anyhow::anyhow;
use ash::vk;
pub use legion;
use legion::World;
use openxr as xr;
use std::{collections::HashMap, io::Seek};

pub use app::App;
pub use gltf_loader::add_model_to_world;
pub use hotham_error::HothamError;
pub use kira::sound::Sound;
pub use scene_data::SceneData;
pub use vertex::Vertex;

// mod animation;
mod app;
mod buffer;
mod camera;
pub mod components;
mod frame;
mod gltf_loader;
mod hand;
mod hotham_error;
mod image;
mod resources;
mod scene_data;
mod schedule_functions;
mod swapchain;
pub mod systems;
mod texture;
pub mod util;
mod vertex;

pub type HothamResult<T> = std::result::Result<T, HothamError>;
pub const COLOR_FORMAT: vk::Format = vk::Format::R8G8B8A8_UNORM;
pub const DEPTH_FORMAT: vk::Format = vk::Format::D32_SFLOAT;
pub const VIEW_COUNT: u32 = 2;
pub const SWAPCHAIN_LENGTH: usize = 3;
pub const VIEW_TYPE: xr::ViewConfigurationType = xr::ViewConfigurationType::PRIMARY_STEREO;
pub const BLEND_MODE: xr::EnvironmentBlendMode = xr::EnvironmentBlendMode::OPAQUE;

#[cfg(target_os = "windows")]
pub const TEXTURE_FORMAT: vk::Format = vk::Format::BC7_SRGB_BLOCK;

#[cfg(target_os = "android")]
pub const TEXTURE_FORMAT: vk::Format = vk::Format::ASTC_4X4_SRGB_BLOCK;

pub trait Program {
    fn get_gltf_data(&self) -> Vec<(&[u8], &[u8])>;
    fn init(&mut self, models: HashMap<String, World>) -> HothamResult<World>;
}

pub fn read_spv_from_bytes<R: std::io::Read + Seek>(bytes: &mut R) -> std::io::Result<Vec<u32>> {
    ash::util::read_spv(bytes)
}

#[cfg(target_os = "windows")]
pub fn load_sound(path: &str) -> HothamResult<Sound> {
    let path = format!("assets/{}", path);
    Sound::from_file(&path, Default::default())
        .map_err(|e| anyhow!("Error loading sound: {} - {:?}", path, e))
        .map_err(|e| HothamError::Other(e))
}

#[cfg(target_os = "android")]
pub fn load_sound(path: &str) -> HothamResult<Sound> {
    use crate::util::get_asset_from_path;

    let asset = get_asset_from_path(path)?;
    let reader = std::io::Cursor::new(asset);
    let p = std::path::Path::new(path);
    let settings = Default::default();
    if let Some(extension) = p.extension() {
        if let Some(extension_str) = extension.to_str() {
            let sound = match extension_str {
                "mp3" => Sound::from_mp3_reader(reader, settings),
                "ogg" => Sound::from_ogg_reader(reader, settings),
                "flac" => Sound::from_flac_reader(reader, settings),
                "wav" => Sound::from_wav_reader(reader, settings),
                _ => Err(kira::sound::error::SoundFromFileError::UnsupportedAudioFileFormat),
            };
            return sound
                .map_err(|e| anyhow!("Error loading sound: {} - {:?}", path, e))
                .map_err(|e| HothamError::Other(e));
        }
    }

    Err(anyhow!("Invalid file formath: {:?}", path)).map_err(|e| HothamError::Other(e))
}
