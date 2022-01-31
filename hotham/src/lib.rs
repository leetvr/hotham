use ash::vk;
use openxr as xr;

pub use engine::Engine;
pub use hecs;
pub use hotham_error::HothamError;
pub use nalgebra;
pub use rapier3d;

mod buffer;
mod camera;
pub mod components;
mod engine;
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
