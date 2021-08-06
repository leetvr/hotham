use crate::node::Node;
use anyhow::anyhow;
use ash::vk;
use openxr as xr;
use std::{cell::RefCell, collections::HashMap, io::Seek, rc::Rc};

pub use app::App;
pub use hotham_error::HothamError;
pub use kira::sound::Sound;
pub use uniform_buffer_object::UniformBufferObject;
pub use vertex::Vertex;

mod animation;
mod app;
mod buffer;
mod camera;
mod frame;
mod gltf_loader;
mod hand;
mod hotham_error;
mod image;
pub mod mesh;
pub mod node;
mod renderer;
mod skin;
mod swapchain;
mod texture;
mod uniform_buffer_object;
mod util;
mod vertex;
mod vulkan_context;

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
    fn get_gltf_data(&self) -> (&[u8], &[u8]);
    fn init(
        &mut self,
        nodes: HashMap<String, Rc<RefCell<Node>>>,
    ) -> HothamResult<ProgramInitialization>;
}

#[derive(Debug, Clone)]
pub struct ProgramInitialization {
    pub nodes: Vec<Rc<RefCell<Node>>>,
    pub sounds: Vec<Sound>,
}

pub fn read_spv_from_bytes<R: std::io::Read + Seek>(bytes: &mut R) -> std::io::Result<Vec<u32>> {
    ash::util::read_spv(bytes)
}

#[cfg(target_os = "windows")]
pub fn load_sound(path: &str) -> HothamResult<Sound> {
    let path = format!("assets/{}", path);
    Sound::from_file(path, Default::default())
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
