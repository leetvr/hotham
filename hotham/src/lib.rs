use std::path::Path;

use anyhow::Result;
use ash::vk;
use hotham_error::HothamError;
use openxr as xr;

pub use app::App;
pub use vertex::Vertex;
pub use view_matrix::ViewMatrix;

mod app;
mod buffer;
mod frame;
mod hotham_error;
mod image;
mod renderer;
mod swapchain;
mod util;
mod vertex;
mod vulkan_context;
mod view_matrix;

pub type HothamResult<T> = std::result::Result<T, HothamError>;
pub const COLOR_FORMAT: vk::Format = vk::Format::R8G8B8A8_UNORM;
pub const DEPTH_FORMAT: vk::Format = vk::Format::D32_SFLOAT;
pub const VIEW_COUNT: u32 = 2;
pub const SWAPCHAIN_LENGTH: usize = 3;
pub const VIEW_TYPE: xr::ViewConfigurationType = xr::ViewConfigurationType::PRIMARY_STEREO;
pub const BLEND_MODE: xr::EnvironmentBlendMode = xr::EnvironmentBlendMode::OPAQUE;

pub trait Program {
    fn update(&self) -> (&Vec<Vertex>, &Vec<u32>);
    fn init(&self) -> ProgramInitialization;
}

#[derive(Debug, Clone)]
pub struct ProgramInitialization<'a> {
    pub vertices: &'a Vec<Vertex>,
    pub indices: &'a Vec<u32>,
    pub vertex_shader: &'a Path,
    pub fragment_shader: &'a Path,
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
