use std::{cell::RefCell, collections::HashMap, os::unix::net::UnixStream, time::Instant};

use ash::vk::{self, Handle};
use hotham_editor_protocol::EditorClient;
use lazy_vulkan::vulkan_context::VulkanContext;
use openxr_sys::{Action, GraphicsBindingVulkanKHR, Instance, Path, Session, SessionState};

use crate::{action_state::ActionState, space_state::SpaceState};

pub struct VulkanCore {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
}

impl VulkanCore {
    pub fn new(entry: ash::Entry, instance: ash::Instance) -> Self {
        Self { entry, instance }
    }
}

pub type SpaceMap = HashMap<u64, SpaceState>;
pub type StringToPathMap = HashMap<String, Path>;
pub type PathToStringMap = HashMap<Path, String>;
pub type BindingMap = HashMap<Path, Action>;

pub struct ClientState {
    // OpenXR stuff
    pub instance: Instance,
    pub session: Session,
    pub session_state: SessionState,
    pub spaces: SpaceMap,
    pub string_to_path: StringToPathMap,
    pub path_to_string: PathToStringMap,
    pub action_state: ActionState,
    pub bindings: BindingMap,
    pub clock: Instant,
    // Only used during initialisation, otherwise None
    pub vulkan_core: Option<VulkanCore>,
    // None during initialisation, otheriwse used during normal session operation
    pub vulkan_context: Option<VulkanContext>,
    pub swapchain_image_count: u32,
    pub swapchain_images: Vec<vk::Image>,
    pub swapchain_semaphores: Vec<vk::Semaphore>,
    // Our connection back to the editor
    pub editor_client: EditorClient<UnixStream>,
}

impl ClientState {
    fn new() -> Self {
        // Connect to the server
        let editor_client = EditorClient::new(
            UnixStream::connect("hotham_editor.socket")
                .expect("hotham_editor.socket was not found!"),
        );

        Self {
            vulkan_core: None,
            vulkan_context: None,
            editor_client,
            instance: Instance::NULL,
            session: Session::NULL,
            spaces: Default::default(),
            string_to_path: Default::default(),
            path_to_string: Default::default(),
            bindings: Default::default(),
            swapchain_image_count: Default::default(),
            swapchain_images: Default::default(),
            swapchain_semaphores: Default::default(),
            session_state: SessionState::UNKNOWN,
            action_state: Default::default(),
            clock: Instant::now(),
        }
    }

    pub fn initialise_vulkan(&mut self, entry: ash::Entry, instance: ash::Instance) {
        self.vulkan_core = Some(VulkanCore::new(entry, instance));
    }

    pub fn vulkan_instance(&self) -> &ash::Instance {
        if let Some(core) = &self.vulkan_core {
            return &core.instance;
        }

        if let Some(context) = &self.vulkan_context {
            return &context.instance;
        }

        panic!("Vulkan hasn't been initialised yet!");
    }

    pub fn create_vulkan_context(&mut self, graphics_binding: &GraphicsBindingVulkanKHR) {
        let VulkanCore { entry, instance } = self
            .vulkan_core
            .take()
            .expect("Vulkan hasn't been initialised yet!");

        let physical_device = vk::PhysicalDevice::from_raw(graphics_binding.physical_device as u64);
        let device = unsafe {
            ash::Device::load(
                instance.fp_v1_0(),
                std::mem::transmute(graphics_binding.device),
            )
        };

        // It's probably fine if the Vulkan context already exists
        self.vulkan_context = Some(VulkanContext::new_with_niche_use_case(
            entry,
            instance,
            physical_device,
            device,
            graphics_binding.queue_family_index,
        ));
    }
}

impl Default for ClientState {
    fn default() -> Self {
        ClientState::new()
    }
}

thread_local! {
    pub static CLIENT_STATE: RefCell<ClientState> = Default::default()
}
