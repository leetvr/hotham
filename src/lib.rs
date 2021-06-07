use anyhow::Result;
use ash::{version::InstanceV1_0, vk};
use cgmath::Vector3;
use hotham_error::HothamError;
use openxr as xr;
use renderer::Renderer;
use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::sleep,
    time::Duration,
};
use xr::{vulkan::SessionCreateInfo, FrameStream, FrameWaiter, Session, Vulkan};

use crate::vulkan_context::VulkanContext;

mod frame;
mod hotham_error;
mod renderer;
mod swapchain;
mod util;
mod vulkan_context;

pub type HothamResult<T> = std::result::Result<T, HothamError>;
pub const COLOR_FORMAT: vk::Format = vk::Format::R8G8B8A8_UNORM;
pub const DEPTH_FORMAT: vk::Format = vk::Format::D32_SFLOAT;
pub const VIEW_COUNT: u32 = 2;
pub const SWAPCHAIN_LENGTH: usize = 3;
pub const VIEW_TYPE: xr::ViewConfigurationType = xr::ViewConfigurationType::PRIMARY_STEREO;

pub struct App<P: Program> {
    program: P,
    should_quit: Arc<AtomicBool>,
    renderer: Renderer,
    _xr_instance: openxr::Instance,
    _xr_session: Session<Vulkan>,
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

impl Vertex {
    pub fn attribute_descriptions() -> Vec<vk::VertexInputAttributeDescription> {
        let position = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32B32A32_SFLOAT)
            .build();

        let colour = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(1)
            .format(vk::Format::R32G32B32A32_SFLOAT)
            .build();

        vec![position, colour]
    }
}

impl<P> App<P>
where
    P: Program,
{
    pub fn new(program: P) -> Result<Self> {
        let params = program.init();
        println!("[HOTHAM_APP] Initialised program with {:?}", params);
        let (xr_instance, system) = create_xr_instance()?;

        let vulkan_context = VulkanContext::create_from_xr_instance(&xr_instance, system)?;
        let (xr_session, _, _) = create_xr_session(&xr_instance, system, &vulkan_context)?;
        let renderer = Renderer::new(vulkan_context, &xr_session, &xr_instance, system, &params)?;

        Ok(Self {
            program,
            renderer,
            should_quit: Arc::new(AtomicBool::from(false)),
            _xr_instance: xr_instance,
            _xr_session: xr_session,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        let should_quit = self.should_quit.clone();
        ctrlc::set_handler(move || should_quit.store(true, Ordering::Relaxed))?;

        while !self.should_quit.load(Ordering::Relaxed) {
            // Tell the program to update its geometry
            let (vertices, indices) = self.program.update();

            // Push the updated geometry back into Vulkan
            self.renderer.update(vertices, indices);

            // Now draw an image
            self.renderer.draw()?;
            sleep(Duration::from_secs(1))
        }

        Ok(())
    }
}

fn create_xr_session(
    xr_instance: &xr::Instance,
    system: xr::SystemId,
    vulkan_context: &VulkanContext,
) -> Result<(Session<Vulkan>, FrameWaiter, FrameStream<Vulkan>)> {
    unsafe {
        xr_instance.create_session(
            system,
            &SessionCreateInfo {
                instance: &vulkan_context.instance.handle() as *const _ as *const _,
                physical_device: &vulkan_context.physical_device as *const _ as *const _,
                device: &vulkan_context.device.handle() as *const _ as *const _,
                queue_family_index: vulkan_context.queue_family_index,
                queue_index: 0,
            },
        )
    }
    .map_err(|e| e.into())
}

fn create_xr_instance() -> anyhow::Result<(xr::Instance, xr::SystemId)> {
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
    Ok((instance, system))
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
