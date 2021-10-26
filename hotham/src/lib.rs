use ash::vk;
pub use legion;
use openxr as xr;

pub use app::App;
pub use hotham_error::HothamError;
pub use rapier3d;
pub use vertex::Vertex;

mod app;
mod buffer;
mod camera;
pub mod components;
mod frame;
pub mod gltf_loader;
mod hotham_error;
mod image;
pub mod resources;
pub mod scene_data;
pub mod schedule_functions;
mod swapchain;
pub mod systems;
mod texture;
pub mod util;
mod vertex;

pub type HothamResult<T> = std::result::Result<T, HothamError>;
pub const COLOR_FORMAT: vk::Format = vk::Format::R8G8B8A8_SRGB;
pub const DEPTH_FORMAT: vk::Format = vk::Format::D32_SFLOAT;
pub const VIEW_COUNT: u32 = 2;
pub const SWAPCHAIN_LENGTH: usize = 3;
pub const VIEW_TYPE: xr::ViewConfigurationType = xr::ViewConfigurationType::PRIMARY_STEREO;
pub const BLEND_MODE: xr::EnvironmentBlendMode = xr::EnvironmentBlendMode::OPAQUE;

pub const DEPTH_ATTACHMENT_USAGE_FLAGS: vk::ImageUsageFlags =
    vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT;

// #[cfg(target_os = "windows")]
// pub fn load_sound(path: &str) -> HothamResult<Sound> {
//     let path = format!("assets/{}", path);
//     Sound::from_file(&path, Default::default())
//         .map_err(|e| anyhow!("Error loading sound: {} - {:?}", path, e))
//         .map_err(|e| HothamError::Other(e))
// }

// #[cfg(target_os = "android")]
// pub fn load_sound(path: &str) -> HothamResult<Sound> {
//     use crate::util::get_asset_from_path;

//     let asset = get_asset_from_path(path)?;
//     let reader = std::io::Cursor::new(asset);
//     let p = std::path::Path::new(path);
//     let settings = Default::default();
//     if let Some(extension) = p.extension() {
//         if let Some(extension_str) = extension.to_str() {
//             let sound = match extension_str {
//                 "mp3" => Sound::from_mp3_reader(reader, settings),
//                 "ogg" => Sound::from_ogg_reader(reader, settings),
//                 "flac" => Sound::from_flac_reader(reader, settings),
//                 "wav" => Sound::from_wav_reader(reader, settings),
//                 _ => Err(kira::sound::error::SoundFromFileError::UnsupportedAudioFileFormat),
//             };
//             return sound
//                 .map_err(|e| anyhow!("Error loading sound: {} - {:?}", path, e))
//                 .map_err(|e| HothamError::Other(e));
//         }
//     }

//     Err(anyhow!("Invalid file formath: {:?}", path)).map_err(|e| HothamError::Other(e))
// }
