use ash::{
    extensions::khr,
    vk::{self, Handle, SwapchainKHR},
    Device, Entry as AshEntry, Instance as AshInstance,
};

use nalgebra::{Quaternion, Unit, UnitQuaternion, Vector3};
use openxr_sys::{Action, Bool32, Path, Posef, SessionState, Space, Vector3f};
use winit::event::KeyboardInput;

use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{
        atomic::{AtomicBool, Ordering::Relaxed},
        mpsc::Receiver,
        Arc,
    },
    thread::JoinHandle,
    time::Instant,
};

use crate::{
    action_state::ActionState, inputs::Inputs, simulator::NUM_VIEWS, space_state::SpaceState,
};

static A_INPUT: &str = "/user/hand/right/input/a/click";
static B_INPUT: &str = "/user/hand/right/input/b/click";
static X_INPUT: &str = "/user/hand/left/input/x/click";
static Y_INPUT: &str = "/user/hand/left/input/y/click";

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
    pub path_string: HashMap<Path, String>,
    pub string_path: HashMap<String, Path>,
    pub spaces: HashMap<u64, SpaceState>,
    pub left_hand_space: u64,
    pub right_hand_space: u64,
    pub view_poses: Vec<Posef>,
    pub keyboard_event_rx: Option<Receiver<KeyboardInput>>,
    pub mouse_event_rx: Option<Receiver<(f64, f64)>>,
    pub input_state: Inputs,
    pub last_frame_time: Instant,
    pub camera: Camera,
    pub action_state: ActionState,
}

#[derive(Default)]
pub struct Camera {
    yaw: f32,
    pitch: f32,
}

impl Default for State {
    fn default() -> Self {
        State {
            camera: Camera::default(),
            vulkan_entry: None,
            vulkan_instance: None,
            physical_device: vk::PhysicalDevice::null(),
            device: None,
            session_state: SessionState::UNKNOWN,
            swapchain_fence: vk::Fence::null(),
            internal_swapchain: SwapchainKHR::null(),
            image_index: 0,
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
            path_string: Default::default(),
            string_path: Default::default(),
            spaces: Default::default(),
            left_hand_space: 0,
            right_hand_space: 0,
            mouse_event_rx: None,
            keyboard_event_rx: None,
            input_state: Inputs::default(),
            last_frame_time: Instant::now(),
            action_state: Default::default(),
            view_poses: (0..NUM_VIEWS)
                .map(|_| {
                    let mut pose = Posef::IDENTITY;
                    pose.position = Vector3f {
                        x: 0.0,
                        y: 1.4,
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

    pub fn update_camera_position(&mut self) -> Option<()> {
        // We need to adjust the speed value so its always the same speed even if the frame rate isn't consistent
        // The delta time is the the current time - last frame time
        let now = Instant::now();
        let delta = now - self.last_frame_time;
        self.last_frame_time = now;
        let dt = delta.as_secs_f32();

        while let Ok(keyboard_input) = self.keyboard_event_rx.as_ref()?.try_recv() {
            self.input_state.process_event(keyboard_input)
        }

        // Camera position & Rotation
        let pose = &mut self.view_poses[0];

        let position = &mut pose.position;
        let orientation = &pose.orientation;
        // get the forward vector rotated by the camera rotation quaternion
        let forward = rotate_vector_by_quaternion(Vector3::new(0f32, 0f32, 1f32), *orientation);
        // get the right vector rotated by the camera rotation quaternion
        let right = rotate_vector_by_quaternion(Vector3::new(1f32, 0f32, 0f32), *orientation);

        let up = Vector3::new(0f32, 1f32, 0f32);
        let movement_speed = 2f32 * dt;
        for pressed in self.input_state.pressed.iter() {
            match pressed {
                winit::event::VirtualKeyCode::W => {
                    position.x -= forward.x * movement_speed;
                    position.y -= forward.y * movement_speed;
                    position.z -= forward.z * movement_speed;
                }
                winit::event::VirtualKeyCode::S => {
                    position.x += forward.x * movement_speed;
                    position.y += forward.y * movement_speed;
                    position.z += forward.z * movement_speed;
                }
                winit::event::VirtualKeyCode::A => {
                    position.x -= right.x * movement_speed;
                    position.y -= right.y * movement_speed;
                    position.z -= right.z * movement_speed;
                }
                winit::event::VirtualKeyCode::D => {
                    position.x += right.x * movement_speed;
                    position.y += right.y * movement_speed;
                    position.z += right.z * movement_speed;
                }
                winit::event::VirtualKeyCode::Space => {
                    position.y += up.y * movement_speed;
                }
                winit::event::VirtualKeyCode::LShift => {
                    position.y -= up.y * movement_speed;
                }
                winit::event::VirtualKeyCode::Q | winit::event::VirtualKeyCode::Escape => {
                    self.session_state = SessionState::EXITING;
                    self.has_event = true;
                }
                _ => {}
            }
        }

        self.view_poses[1] = self.view_poses[0];
        Some(())
    }

    /// Updates the OpenXR camera Position & Rotation
    /// Tries to emulate a simple first person floating camera to help navigate the scene
    pub fn update_camera_rotation(&mut self) -> Option<()> {
        let mut x_rot = 0f32;
        let mut y_rot = 0f32;

        while let Ok((x, y)) = self.mouse_event_rx.as_ref()?.try_recv() {
            x_rot -= x as f32;
            y_rot -= y as f32;
        }

        // Camera position & Rotation
        let pose = &mut self.view_poses[0];

        // Update Rotation
        let orientation = &mut pose.orientation;

        let mouse_sensitivity = 0.2 * std::f32::consts::TAU / 360f32;

        self.camera.yaw += x_rot * mouse_sensitivity;
        self.camera.pitch += y_rot * mouse_sensitivity;

        // I think I'm converting these two types incorrectly but it seems to work and I'm too scared to break it lol
        let rotation: Unit<Quaternion<f32>> =
            UnitQuaternion::from_euler_angles(self.camera.pitch, self.camera.yaw, 0f32);

        orientation.x = rotation.i;
        orientation.y = rotation.j;
        orientation.z = rotation.k;
        orientation.w = rotation.w;

        self.view_poses[1] = self.view_poses[0];
        Some(())
    }

    /// Update simulated action state
    pub fn update_action_state(&mut self) {
        // Reset the state of all the inputs
        self.action_state.clear();

        // Clone to get around mutate after borrow
        for pressed in &self.input_state.clone().pressed {
            match pressed {
                winit::event::VirtualKeyCode::Key1 => {
                    self.press(X_INPUT);
                }
                winit::event::VirtualKeyCode::Key2 => {
                    self.press(Y_INPUT);
                }
                winit::event::VirtualKeyCode::Key3 => {
                    self.press(B_INPUT);
                }
                winit::event::VirtualKeyCode::Key4 => {
                    self.press(A_INPUT);
                }
                _ => {}
            }
        }
    }

    /// Checks to see whether a specific action has been triggered.
    pub fn get_action_state_boolean(&self, action: Action) -> Bool32 {
        self.action_state.get_boolean(action)
    }

    fn press(&mut self, path_string: &str) {
        let path = self.string_path.get(path_string).unwrap();
        self.action_state.set_boolean(path, true);
    }
}

/// Rotates a Vector by the quaternion
/// Used to get the forward, right direction to control the camera
fn rotate_vector_by_quaternion(
    vector: Vector3<f32>,
    openxr_sys::Quaternionf { x, y, z, w }: openxr_sys::Quaternionf,
) -> Vector3f {
    let i = x;
    let j = y;
    let k = z;
    let w = w;
    let q1: Unit<Quaternion<f32>> = UnitQuaternion::from_quaternion(Quaternion::new(w, i, j, k));

    let rotated_vector = q1.transform_vector(&vector);

    Vector3f {
        x: rotated_vector.x,
        y: rotated_vector.y,
        z: rotated_vector.z,
    }
}
