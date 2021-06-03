use ash::vk;
use cgmath::Vector3;
use hotham_error::HothamError;
use openxr as xr;
use renderer::Renderer;
use std::{path::Path, thread::sleep, time::Duration};

use crate::vulkan_context::VulkanContext;

mod hotham_error;
mod renderer;
mod swapchain;
mod util;
mod vulkan_context;

pub type Result<T> = std::result::Result<T, HothamError>;
pub const COLOR_FORMAT: vk::Format = vk::Format::R8G8B8A8_UNORM;
pub const VIEW_COUNT: u32 = 2;
pub const VIEW_TYPE: xr::ViewConfigurationType = xr::ViewConfigurationType::PRIMARY_STEREO;

#[derive(Clone)]
pub struct App<P: Program> {
    program: P,
    should_quit: bool,
    renderer: Renderer,
    instance: openxr::Instance,
}

#[derive(Clone, Debug)]
pub struct Vertex {
    position: Vector3<f32>,
    color: Vector3<f32>,
}

impl Vertex {
    pub fn new(position: Vector3<f32>, color: Vector3<f32>) -> Self {
        Self { position, color }
    }
}

impl<P> App<P>
where
    P: Program,
{
    pub fn new(program: P) -> Result<Self> {
        let params = program.init();
        println!("Initialised program with {:?}", params);
        let xr_entry = xr::Entry::linked();
        let xr_app_info = openxr::ApplicationInfo {
            application_name: "Hotham Cubeworld",
            application_version: 1,
            engine_name: "Hotham",
            engine_version: 1,
        };
        let mut required_extensions = xr::ExtensionSet::default();
        required_extensions.khr_vulkan_enable2 = true;
        let instance = xr_entry.create_instance(&xr_app_info, &required_extensions, &[])?;
        let system = instance.system(xr::FormFactor::HEAD_MOUNTED_DISPLAY)?;
        let _environment_blend_mode =
            instance.enumerate_environment_blend_modes(system, VIEW_TYPE)?[0];

        let vk_target_version_xr = xr::Version::new(1, 1, 0);

        let requirements = instance.graphics_requirements::<xr::Vulkan>(system)?;
        if vk_target_version_xr < requirements.min_api_version_supported
            || vk_target_version_xr.major() > requirements.max_api_version_supported.major()
        {
            return Err(HothamError::UnsupportedVersionError);
        }

        let vulkan_context = VulkanContext::create_from_xr_instance(&instance, system)?;

        Ok(Self {
            program,
            renderer: Renderer::new(vulkan_context),
            should_quit: false,
            instance,
        })
    }

    pub fn run(&self) -> () {
        while !&self.should_quit {
            // Tell the program to update its geometry
            let (vertices, indices) = self.program.update();

            // Push the updated geometry back into Vulkan
            self.renderer.update(vertices, indices);
            sleep(Duration::from_secs(1))
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProgramInitialization<'a> {
    pub vertices: &'a Vec<Vertex>,
    pub indices: &'a Vec<u32>,
    pub vertex_shader: &'a Path,
    pub fragment_shader: &'a Path,
}

pub trait Program {
    fn update(&self) -> (&Vec<Vertex>, &Vec<u32>);
    fn init(&self) -> ProgramInitialization;
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
