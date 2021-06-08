use anyhow::Result;
use ash::{version::InstanceV1_0, vk};
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
use xr::{
    vulkan::SessionCreateInfo, EventDataBuffer, FrameStream, FrameWaiter, Session, SessionState,
    Swapchain, SwapchainCreateFlags, SwapchainCreateInfo, SwapchainUsageFlags, Vulkan,
};

pub use vertex::Vertex;

use crate::vulkan_context::VulkanContext;

mod buffer;
mod frame;
mod hotham_error;
mod image;
mod renderer;
mod swapchain;
mod util;
mod vertex;
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
    xr_instance: openxr::Instance,
    xr_session: Session<Vulkan>,
    xr_state: SessionState,
    xr_swapchain: Swapchain<Vulkan>,
    event_buffer: EventDataBuffer,
    frame_waiter: FrameWaiter,
    frame_stream: FrameStream<Vulkan>,
}

impl<P> App<P>
where
    P: Program,
{
    pub fn new(program: P) -> HothamResult<Self> {
        let params = program.init();
        println!("[HOTHAM_APP] Initialised program with {:?}", params);
        let (xr_instance, system) = create_xr_instance()?;

        let vulkan_context = VulkanContext::create_from_xr_instance(&xr_instance, system)?;
        let (xr_session, frame_waiter, frame_stream) =
            create_xr_session(&xr_instance, system, &vulkan_context)?; // TODO: Extract to XRContext
        let swapchain_resolution = get_swapchain_resolution(&xr_instance, system)?;
        let xr_swapchain = create_xr_swapchain(&xr_session, &swapchain_resolution)?;

        let renderer = Renderer::new(
            vulkan_context,
            &xr_session,
            &xr_instance,
            &xr_swapchain,
            swapchain_resolution,
            system,
            &params,
        )?;

        Ok(Self {
            program,
            renderer,
            should_quit: Arc::new(AtomicBool::from(false)),
            xr_instance,
            xr_session,
            xr_swapchain,
            xr_state: SessionState::IDLE,
            event_buffer: Default::default(),
            frame_stream,
            frame_waiter,
        })
    }

    pub fn run(&mut self) -> HothamResult<()> {
        let should_quit = self.should_quit.clone();
        ctrlc::set_handler(move || should_quit.store(true, Ordering::Relaxed))
            .map_err(anyhow::Error::new)?;

        while !self.should_quit.load(Ordering::Relaxed) {
            let current_state = self.poll_xr_event()?;
            if current_state == SessionState::IDLE {
                sleep(Duration::from_secs(1));
                continue;
            }

            if current_state == SessionState::EXITING {
                break;
            }

            // Tell the program to update its geometry
            let (vertices, indices) = self.program.update();

            // Push the updated geometry back into Vulkan
            self.renderer.update(vertices, indices);

            // Wait for a frame to become available from the runtime
            let frame_state = self.frame_waiter.wait()?;
            if frame_state.should_render {
                // Now draw an image
                self.renderer.draw()?;
            }
        }

        Ok(())
    }

    fn poll_xr_event(&mut self) -> Result<SessionState> {
        loop {
            match self.xr_instance.poll_event(&mut self.event_buffer)? {
                Some(xr::Event::SessionStateChanged(session_changed)) => {
                    let new_state = session_changed.state();

                    if self.xr_state == SessionState::IDLE && new_state == SessionState::READY {
                        self.xr_session.begin(VIEW_TYPE)?;
                    }

                    println!("[HOTHAM_POLL_EVENT] State is now {:?}", new_state);
                    self.xr_state = new_state;
                }
                Some(_) => {}
                None => break,
            }
        }

        Ok(self.xr_state)
    }
}

fn get_swapchain_resolution(
    xr_instance: &xr::Instance,
    system: xr::SystemId,
) -> Result<vk::Extent2D> {
    let views = xr_instance.enumerate_view_configuration_views(system, VIEW_TYPE)?;
    let resolution = vk::Extent2D {
        width: views[0].recommended_image_rect_width,
        height: views[0].recommended_image_rect_height,
    };

    Ok(resolution)
}

fn create_xr_swapchain(
    xr_session: &Session<Vulkan>,
    resolution: &vk::Extent2D,
) -> Result<Swapchain<Vulkan>> {
    xr_session
        .create_swapchain(&SwapchainCreateInfo {
            create_flags: SwapchainCreateFlags::EMPTY,
            usage_flags: SwapchainUsageFlags::COLOR_ATTACHMENT,
            format: COLOR_FORMAT.as_raw() as u32,
            sample_count: 1,
            width: resolution.width,
            height: resolution.height,
            face_count: 1,
            array_size: VIEW_COUNT,
            mip_count: 1,
        })
        .map_err(Into::into)
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
