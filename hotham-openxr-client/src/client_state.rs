use std::{cell::RefCell, collections::HashMap, os::unix::net::UnixStream, time::Instant};

use ash::vk;
use hotham_editor_protocol::EditorClient;
use lazy_vulkan::vulkan_context::VulkanContext;
use openxr_sys::{Action, Instance, Path, Session, SessionState};

use crate::{action_state::ActionState, space_state::SpaceState};

pub struct VulkanCore {
    entry: ash::Entry,
    instance: ash::Instance,
}

pub type SpaceMap = HashMap<u64, SpaceState>;
pub type StringToPathMap = HashMap<String, Path>;
pub type PathToStringMap = HashMap<Path, String>;
pub type BindingMap = HashMap<Path, Action>;

struct ClientState {
    instance: Instance,
    session: Session,
    // Only used during initialisation, otherwise None
    vulkan_core: Option<VulkanCore>,
    // None during initialisation, otheriwse used during normal session operation
    vulkan_context: Option<VulkanContext>,
    spaces: SpaceMap,
    editor_client: EditorClient<UnixStream>,
    string_to_path: StringToPathMap,
    path_to_string: StringToPathMap,
    bindings: BindingMap,
    swapchain_image_count: u32,
    swapchain_images: Vec<vk::Image>,
    swapchain_semaphores: Vec<vk::Semaphore>,
    session_state: SessionState,
    action_state: ActionState,
    clock: Instant,
}

impl ClientState {
    fn new() -> Self {
        todo!()
        // Self {
        //     instance,
        //     session,
        //     vulkan_core,
        //     spaces,
        //     editor_client,
        //     string_to_path,
        //     path_to_string,
        //     bindings,
        //     swapchain_image_count,
        //     swapchain_images,
        //     swapchain_semaphores,
        //     session_state,
        //     action_state,
        //     clock,
        // }
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
