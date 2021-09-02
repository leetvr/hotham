use ash::{
    extensions::khr,
    vk::{self, Handle, SwapchainKHR},
    Device, Entry as AshEntry, Instance as AshInstance,
};

use openxr_sys::{Path, Posef, SessionState, Space, Vector3f};

use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{
        atomic::{AtomicBool, Ordering::Relaxed},
        mpsc::Receiver,
        Arc,
    },
    thread::JoinHandle,
};

use crate::{
    simulator::{HothamInputEvent, NUM_VIEWS},
    space_state::SpaceState,
};
// use crate::simulator::spa
pub struct State {
    pub vulkan_entry: Option<AshEntry>,
    pub vulkan_instance: Option<AshInstance>,
    pub physical_device: vk::PhysicalDevice,
    pub device: Option<Device>,
    pub session_state: SessionState,
    pub swapchain_fence: vk::Fence,
    pub internal_swapchain: SwapchainKHR,
    pub internal_swapchain_images: Vec<vk::Image>,
    pub internal_swapchain_image_views: Vec<vk::ImageView>,
    pub frame_count: usize,
    pub image_index: u32,
    pub present_queue: vk::Queue,
    pub present_queue_family_index: u32,
    pub command_pool: vk::CommandPool,
    pub multiview_images: Vec<vk::Image>,
    pub multiview_image_views: Vec<vk::ImageView>,
    pub multiview_images_memory: Vec<vk::DeviceMemory>,
    pub has_event: bool,
    pub command_buffers: Vec<vk::CommandBuffer>,
    pub pipelines: Vec<vk::Pipeline>,
    pub render_pass: vk::RenderPass,
    pub pipeline_layout: vk::PipelineLayout,
    pub descriptor_sets: Vec<vk::DescriptorSet>,
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub framebuffers: Vec<vk::Framebuffer>,
    pub render_complete_semaphores: Vec<vk::Semaphore>,
    pub close_window: Arc<AtomicBool>,
    pub sampler: vk::Sampler,
    pub descriptor_pool: vk::DescriptorPool,
    pub surface: vk::SurfaceKHR,
    pub window_thread_handle: Option<JoinHandle<()>>,
    pub reference_space: Space,
    pub paths: HashMap<Path, String>,
    pub spaces: HashMap<u64, SpaceState>,
    pub left_hand_space: u64,
    pub right_hand_space: u64,
    pub view_poses: Vec<Posef>,
    pub event_rx: Option<Receiver<HothamInputEvent>>,
}

impl Default for State {
    fn default() -> Self {
        State {
            vulkan_entry: None,
            vulkan_instance: None,
            physical_device: vk::PhysicalDevice::null(),
            device: None,
            session_state: SessionState::UNKNOWN,
            swapchain_fence: vk::Fence::null(),
            internal_swapchain: SwapchainKHR::null(),
            image_index: 4,
            present_queue: vk::Queue::null(),
            present_queue_family_index: 0,
            command_pool: vk::CommandPool::null(),
            internal_swapchain_images: Vec::new(),
            multiview_images: Vec::new(),
            multiview_images_memory: Vec::new(),
            frame_count: 0,
            has_event: false,
            internal_swapchain_image_views: Default::default(),
            multiview_image_views: Default::default(),
            command_buffers: Default::default(),
            pipelines: Default::default(),
            render_pass: Default::default(),
            pipeline_layout: Default::default(),
            descriptor_sets: Default::default(),
            descriptor_set_layout: Default::default(),
            framebuffers: Default::default(),
            render_complete_semaphores: Default::default(),
            close_window: Default::default(),
            sampler: vk::Sampler::null(),
            descriptor_pool: vk::DescriptorPool::null(),
            surface: vk::SurfaceKHR::null(),
            window_thread_handle: None,
            reference_space: Space::NULL,
            paths: Default::default(),
            spaces: Default::default(),
            left_hand_space: 0,
            right_hand_space: 0,
            event_rx: None,
            view_poses: (0..NUM_VIEWS)
                .map(|_| {
                    let mut pose = Posef::IDENTITY;
                    pose.position = Vector3f {
                        x: 0.0,
                        y: 1.8,
                        z: 0.0,
                    };
                    pose
                })
                .collect(),
        }
    }
}

impl Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("State")
            .field(
                "vulkan_instance",
                &self
                    .vulkan_instance
                    .as_ref()
                    .map(|i| i.handle().as_raw())
                    .unwrap_or(0),
            )
            .finish()
    }
}

impl State {
    pub unsafe fn destroy(&mut self) {
        println!("[HOTHAM_SIMULATOR] Destroy called..");
        if let Some(device) = self.device.take() {
            let instance = self.vulkan_instance.take().unwrap();
            let entry = self.vulkan_entry.take().unwrap();
            device.device_wait_idle().unwrap();
            self.close_window.store(true, Relaxed);
            self.window_thread_handle.take().unwrap().join().unwrap();

            let swapchain_ext = khr::Swapchain::new(&instance, &device);
            swapchain_ext.destroy_swapchain(self.internal_swapchain, None);

            let surface_ext = khr::Surface::new(&entry, &instance);
            surface_ext.destroy_surface(self.surface, None);
        }
        // device.queue_wait_idle(self.present_queue).unwrap();
        // for image in self.multiview_images.drain(..) {
        //     device.destroy_image(image, None)
        // }
        // device.destroy_fence(self.swapchain_fence, None);
        // for memory in self.multiview_images_memory.drain(..) {
        //     device.free_memory(memory, None)
        // }

        // device.destroy_command_pool(self.command_pool, None);
        // for semaphore in self.render_complete_semaphores.drain(..) {
        //     device.destroy_semaphore(semaphore, None)
        // }

        // for image_view in self.internal_swapchain_image_views.drain(..) {
        //     device.destroy_image_view(image_view, None)
        // }

        // for image_view in self.multiview_image_views.drain(..) {
        //     device.destroy_image_view(image_view, None)
        // }

        // device.destroy_pipeline_layout(self.pipeline_layout, None);
        // for pipeline in self.pipelines.drain(..) {
        //     device.destroy_pipeline(pipeline, None);
        // }

        // device.destroy_render_pass(self.render_pass, None);
        // device.destroy_sampler(self.sampler, None);
        // device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        // device.destroy_descriptor_pool(self.descriptor_pool, None);
        // for framebuffer in self.framebuffers.drain(..) {
        //     device.destroy_framebuffer(framebuffer, None)
        // }

        // device.destroy_device(None);
        // instance.destroy_instance(None);

        println!("[HOTHAM_SIMULATOR] All things are now destroyed");
    }

    pub fn update_actions(&mut self) -> Option<()> {
        let mut z_delta = 0.00;
        let mut x_delta = 0.00;
        let mut x_rot = 0.0;
        let mut y_rot = 0.0;

        match self.event_rx.as_ref()?.try_recv().ok()? {
            HothamInputEvent::KeyboardInput { key } => match key? {
                winit::event::VirtualKeyCode::W => {
                    z_delta = -0.05;
                }
                winit::event::VirtualKeyCode::S => {
                    z_delta = 0.05;
                }
                winit::event::VirtualKeyCode::A => {
                    x_delta = -0.05;
                }
                winit::event::VirtualKeyCode::D => {
                    x_delta = 0.05;
                }
                _ => {}
            },
            HothamInputEvent::MouseInput { x, y } => {
                x_rot = -(x * 0.001) as _;
                y_rot = -(y * 0.001) as _;
            }
        }

        let pose = &mut self.view_poses[0];
        let orientation = &mut pose.orientation;
        orientation.x = (orientation.x + x_rot).clamp(-1.0, 1.0);
        orientation.y = (orientation.y + y_rot).clamp(-1.0, 1.0);

        let position = &mut pose.position;
        position.z += z_delta;
        position.x += x_delta;

        // let left_hand = self.left_hand_space;
        // let right_hand = self.right_hand_space;
        // self.spaces.get_mut(&left_hand).unwrap().position.z += z_delta;
        // self.spaces.get_mut(&left_hand).unwrap().position.x += x_delta;
        // self.spaces.get_mut(&right_hand).unwrap().position.z += z_delta;
        // self.spaces.get_mut(&right_hand).unwrap().position.x += x_delta;

        Some(())
    }
}
