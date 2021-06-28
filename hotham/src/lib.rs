use ash::vk;
use hotham_error::HothamError;
use openxr as xr;
use std::io::Seek;

pub use app::App;
pub use uniform_buffer_object::UniformBufferObject;
pub use vertex::Vertex;

mod app;
mod buffer;
mod camera;
mod frame;
mod hotham_error;
mod image;
mod renderer;
mod swapchain;
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
pub trait Program {
    fn update(&mut self) -> (&Vec<Vertex>, &Vec<u32>);
    fn init(&mut self) -> HothamResult<ProgramInitialization>;
}

#[derive(Debug, Clone)]
pub struct ProgramInitialization<'a> {
    pub vertices: &'a Vec<Vertex>,
    pub indices: &'a Vec<u32>,
    pub vertex_shader: Vec<u32>,
    pub fragment_shader: Vec<u32>,
    pub image_buf: Vec<u8>,
    pub image_height: u32,
    pub image_width: u32,
}

pub fn read_spv_from_bytes<R: std::io::Read + Seek>(bytes: &mut R) -> std::io::Result<Vec<u32>> {
    ash::util::read_spv(bytes)
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
