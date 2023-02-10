#![allow(
    non_snake_case,
    dead_code,
    non_upper_case_globals,
    non_camel_case_types
)]
use crate::space_state::SpaceState;
use ash::vk::{self, Handle};
use hotham_editor_protocol::{requests, responses, EditorClient, Request};
use log::{debug, error, trace};
use once_cell::sync::OnceCell;
use openxr_sys::{
    platform::{VkDevice, VkInstance, VkPhysicalDevice, VkResult},
    Action, ActionCreateInfo, ActionSet, ActionSetCreateInfo, ActionSpaceCreateInfo,
    ActionStateBoolean, ActionStateFloat, ActionStateGetInfo, ActionStatePose, ActionsSyncInfo,
    EnvironmentBlendMode, EventDataBuffer, ExtensionProperties, FrameBeginInfo, FrameEndInfo,
    FrameState, FrameWaitInfo, GraphicsBindingVulkanKHR, GraphicsRequirementsVulkanKHR,
    HapticActionInfo, HapticBaseHeader, Instance, InstanceCreateInfo, InstanceProperties,
    InteractionProfileSuggestedBinding, Path, ReferenceSpaceCreateInfo, ReferenceSpaceType, Result,
    Session, SessionActionSetsAttachInfo, SessionBeginInfo, SessionCreateInfo, Space,
    SpaceLocation, StructureType, Swapchain, SwapchainCreateInfo, SwapchainImageAcquireInfo,
    SwapchainImageBaseHeader, SwapchainImageReleaseInfo, SwapchainImageWaitInfo, SystemGetInfo,
    SystemId, SystemProperties, Time, Version, View, ViewConfigurationType, ViewConfigurationView,
    ViewLocateInfo, ViewState, VulkanDeviceCreateInfoKHR, VulkanGraphicsDeviceGetInfoKHR,
    VulkanInstanceCreateInfoKHR, FALSE, TRUE,
};

use lazy_vulkan::vulkan_context::VulkanContext;
use std::{
    collections::HashMap,
    ffi::{c_char, CStr},
    mem::transmute,
    ptr::null_mut,
};
use std::{ptr, slice};
use uds_windows::UnixStream;

type PartialVulkan = (ash::Entry, ash::Instance);
type SpaceMap = HashMap<u64, SpaceState>;

// Used during the init phase
static mut PARTIAL_VULKAN: OnceCell<PartialVulkan> = OnceCell::new();
static INSTANCE: OnceCell<Instance> = OnceCell::new();
static SESSION: OnceCell<Session> = OnceCell::new();
static VULKAN_CONTEXT: OnceCell<VulkanContext> = OnceCell::new();
static mut SPACES: OnceCell<SpaceMap> = OnceCell::new();
static mut EDITOR_CLIENT: OnceCell<EditorClient<UnixStream>> = OnceCell::new();

pub unsafe extern "system" fn enumerate_instance_extension_properties(
    _layer_names: *const ::std::os::raw::c_char,
    property_capacity_input: u32,
    property_count_output: *mut u32,
    properties: *mut ExtensionProperties,
) -> Result {
    // If the client didn't initialise env logger, do it now
    let _ = env_logger::builder()
        .filter_module("hotham_openxr_client", log::LevelFilter::Trace)
        .try_init();

    trace!("enumerate_instance_extension_properties");

    set_array(
        property_capacity_input,
        property_count_output,
        properties,
        [
            ExtensionProperties {
                ty: StructureType::EXTENSION_PROPERTIES,
                next: ptr::null_mut(),
                extension_name: str_to_fixed_bytes("XR_KHR_vulkan_enable"),
                extension_version: 1,
            },
            ExtensionProperties {
                ty: StructureType::EXTENSION_PROPERTIES,
                next: ptr::null_mut(),
                extension_name: str_to_fixed_bytes("XR_KHR_vulkan_enable2"),
                extension_version: 1,
            },
        ],
    );
    Result::SUCCESS
}

pub unsafe extern "system" fn create_instance(
    _create_info: *const InstanceCreateInfo,
    instance: *mut Instance,
) -> Result {
    // If the client didn't initialise env logger, do it now
    let _ = env_logger::builder()
        .filter_module("hotham_openxr_client", log::LevelFilter::Trace)
        .try_init();

    trace!("create_instance");

    // Initialise our spaces map
    let _ = SPACES.set(Default::default());

    // New instance, new luck.
    *instance = Instance::from_raw(rand::random());
    let _ = INSTANCE.set(*instance);

    // Connect to the server
    match UnixStream::connect("hotham_editor.socket") {
        Ok(stream) => {
            trace!("Successfully connected to editor!");
            drop(EDITOR_CLIENT.set(EditorClient::new(stream)));
            Result::SUCCESS
        }
        Err(e) => {
            error!("Unable to establish connection to editor: {e:?}");
            Result::ERROR_INITIALIZATION_FAILED
        }
    }
}

pub unsafe extern "system" fn get_system(
    _instance: Instance,
    _get_info: *const SystemGetInfo,
    system_id: *mut SystemId,
) -> Result {
    trace!("get_system");
    // we are teh leetzor systemz
    *system_id = SystemId::from_raw(1337);
    Result::SUCCESS
}

pub unsafe extern "system" fn create_vulkan_instance(
    _instance: Instance,
    create_info: *const VulkanInstanceCreateInfoKHR,
    vulkan_instance: *mut VkInstance,
    vulkan_result: *mut VkResult,
) -> Result {
    trace!("create_vulkan_instance");

    // I mean, look, we're *meant* to use the pfnGetInstanceProcAddr from the client
    // but what are the odds that it's going to be any different from ours?
    //
    // We do care about the extensions though - they're important.
    let create_info = *create_info;

    let instance_create_info: &vk::InstanceCreateInfo = &(*create_info.vulkan_create_info.cast());

    let extension_names = if instance_create_info.enabled_extension_count > 0 {
        std::slice::from_raw_parts(
            instance_create_info.pp_enabled_extension_names,
            instance_create_info.enabled_extension_count as _,
        )
    } else {
        &[]
    };

    trace!("Application requested extension names: {extension_names:?}");

    let (entry, instance) = lazy_vulkan::vulkan_context::init(&mut extension_names.to_vec());
    *vulkan_instance = transmute(instance.handle().as_raw());

    let _ = PARTIAL_VULKAN.set((entry, instance));

    *vulkan_result = vk::Result::SUCCESS.as_raw();
    Result::SUCCESS
}

pub unsafe extern "system" fn get_vulkan_graphics_device_2(
    _instance: Instance,
    _get_info: *const VulkanGraphicsDeviceGetInfoKHR,
    vulkan_physical_device: *mut VkPhysicalDevice,
) -> Result {
    trace!("get_vulkan_graphics_device_2");
    let (_, instance) = PARTIAL_VULKAN.get().unwrap();
    let physical_device = lazy_vulkan::vulkan_context::get_physical_device(instance, None, None).0;
    trace!("Physical device: {physical_device:?}");
    *vulkan_physical_device = transmute(physical_device.as_raw());
    Result::SUCCESS
}

pub unsafe extern "system" fn create_vulkan_device(
    _instance: Instance,
    create_info: *const VulkanDeviceCreateInfoKHR,
    vulkan_device: *mut VkDevice,
    vulkan_result: *mut VkResult,
) -> Result {
    trace!("create_vulkan_device");
    let (_, instance) = PARTIAL_VULKAN.get().unwrap();
    let create_info = &*create_info;
    let physical_device: vk::PhysicalDevice =
        vk::PhysicalDevice::from_raw(transmute(create_info.vulkan_physical_device));
    let device_create_info: &vk::DeviceCreateInfo = &*create_info.vulkan_create_info.cast();
    trace!("Physical device: {physical_device:?}");
    trace!("Create info: {device_create_info:?}");

    // Create a Vulkan device for the *application*. We'll stash our own away soon enough,
    // don't you worry about that.
    let device = instance
        .create_device(physical_device, &vk::DeviceCreateInfo::builder(), None)
        .unwrap();

    *vulkan_device = transmute(device.handle().as_raw());

    *vulkan_result = vk::Result::SUCCESS.as_raw();
    Result::SUCCESS
}

pub unsafe extern "system" fn get_vulkan_physical_device(
    _instance: Instance,
    _system_id: SystemId,
    _vk_instance: VkInstance,
    _vk_physical_device: *mut VkPhysicalDevice,
) -> Result {
    trace!("get_vulkan_physical_device");
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn get_vulkan_graphics_requirements(
    _instance: Instance,
    _system_id: SystemId,
    graphics_requirements: *mut GraphicsRequirementsVulkanKHR,
) -> Result {
    trace!("get_vulkan_graphics_requirements");
    *graphics_requirements = GraphicsRequirementsVulkanKHR {
        ty: GraphicsRequirementsVulkanKHR::TYPE,
        next: ptr::null_mut(),
        min_api_version_supported: Version::new(1, 1, 0),
        max_api_version_supported: Version::new(1, 3, 0),
    };
    Result::SUCCESS
}

pub unsafe extern "system" fn get_instance_properties(
    _instance: Instance,
    instance_properties: *mut InstanceProperties,
) -> Result {
    trace!("get_instance_properties");
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn enumerate_environment_blend_modes(
    _instance: Instance,
    _system_id: SystemId,
    _view_configuration_type: ViewConfigurationType,
    environment_blend_mode_capacity_input: u32,
    environment_blend_mode_count_output: *mut u32,
    environment_blend_modes: *mut EnvironmentBlendMode,
) -> Result {
    trace!("enumerate_environment_blend_modes");
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn create_session(
    _instance: Instance,
    create_info: *const SessionCreateInfo,
    session: *mut Session,
) -> Result {
    trace!("create_session");
    *session = Session::from_raw(rand::random());
    let _ = SESSION.set(*session); // TODO: I'm not sure if it should be an error to create a new session again
    let graphics_binding = &*((*create_info).next as *const GraphicsBindingVulkanKHR);
    let (entry, instance) = PARTIAL_VULKAN.take().unwrap();
    let physical_device = vk::PhysicalDevice::from_raw(transmute(graphics_binding.physical_device));
    let device = ash::Device::load(instance.fp_v1_0(), transmute(graphics_binding.device));
    let queue_family_index = graphics_binding.queue_family_index;

    // It's probably fine if the vulkan context already exists
    let _ = VULKAN_CONTEXT.set(VulkanContext::new_with_niche_use_case(
        entry,
        instance,
        physical_device,
        device,
        queue_family_index,
    ));

    Result::SUCCESS
}

pub unsafe extern "system" fn create_action_set(
    _instance: Instance,
    create_info: *const ActionSetCreateInfo,
    action_set: *mut ActionSet,
) -> Result {
    trace!("create_action_set");
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn create_action(
    _action_set: ActionSet,
    _create_info: *const ActionCreateInfo,
    action_out: *mut Action,
) -> Result {
    trace!("create_action");
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn suggest_interaction_profile_bindings(
    _instance: Instance,
    suggested_bindings: *const InteractionProfileSuggestedBinding,
) -> Result {
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn string_to_path(
    _instance: Instance,
    path_string: *const c_char,
    path_out: *mut Path,
) -> Result {
    match CStr::from_ptr(path_string).to_str() {
        Ok(s) => {
            let path = Path::from_raw(rand::random());
            Result::ERROR_FEATURE_UNSUPPORTED
        }
        Err(_) => Result::ERROR_VALIDATION_FAILURE,
    }
}

pub unsafe extern "system" fn attach_action_sets(
    _session: Session,
    _attach_info: *const SessionActionSetsAttachInfo,
) -> Result {
    println!("[HOTHAM_SIMULATOR] Attach action sets called");
    Result::ERROR_FEATURE_UNSUPPORTED
}

// TODO: Handle aim pose.
pub unsafe extern "system" fn create_action_space(
    _session: Session,
    create_info: *const ActionSpaceCreateInfo,
    space_out: *mut Space,
) -> Result {
    // match state
    //     .path_string
    //     .get(&(*create_info).subaction_path)
    //     .map(|s| s.as_str())
    // {
    // Some("/user/hand/left") => {
    //     let mut space_state = SpaceState::new("Left Hand");
    //     space_state.position = Vector3f {
    //         x: -0.20,
    //         y: 1.4,
    //         z: -0.50,
    //     };
    //     space_state.orientation = Quaternionf {
    //         x: 0.707,
    //         y: 0.,
    //         z: 0.,
    //         w: 0.707,
    //     };
    //     println!("[HOTHAM_SIMULATOR] Created left hand space: {space_state:?}, {space:?}");
    //     state.left_hand_space = raw;
    //     state.spaces.insert(raw, space_state);
    // }
    // Some("/user/hand/right") => {
    //     let mut space_state = SpaceState::new("Right Hand");
    //     space_state.orientation = Quaternionf {
    //         x: 0.707,
    //         y: 0.,
    //         z: 0.,
    //         w: 0.707,
    //     };
    //     space_state.position = Vector3f {
    //         x: 0.20,
    //         y: 1.4,
    //         z: -0.50,
    //     };
    //     println!("[HOTHAM_SIMULATOR] Created right hand space: {space_state:?}, {space:?}");
    //     state.right_hand_space = raw;
    //     state.spaces.insert(raw, space_state);
    // }
    // Some(path) => {
    //     let space_state = SpaceState::new(path);
    //     println!("[HOTHAM_SIMULATOR] Created space for path: {path}");
    //     state.spaces.insert(raw, space_state);
    // }
    // _ => {}
    // }

    // *space_out = space;
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn create_reference_space(
    _session: Session,
    create_info: *const ReferenceSpaceCreateInfo,
    out_space: *mut Space,
) -> Result {
    trace!("create_reference_space");
    let create_info = *create_info;

    // Our "reference space" is Stage with no rotation
    let (reference_space, mut space_state) = if create_info.reference_space_type
        == ReferenceSpaceType::STAGE
        && create_info.pose_in_reference_space.orientation.w != 1.0
    {
        // Magic value
        (Space::from_raw(0), SpaceState::new("Stage"))
    } else {
        (Space::from_raw(rand::random()), SpaceState::new("View"))
    };

    space_state.position = create_info.pose_in_reference_space.position;
    space_state.orientation = create_info.pose_in_reference_space.orientation;

    SPACES
        .get_mut()
        .unwrap()
        .insert(reference_space.into_raw(), space_state);

    *out_space = reference_space;
    Result::SUCCESS
}

pub unsafe extern "system" fn poll_event(
    _instance: Instance,
    event_data: *mut EventDataBuffer,
) -> Result {
    // let mut state = STATE.lock().unwrap();
    // let mut next_state = state.session_state;
    // if state.session_state == SessionState::UNKNOWN {
    //     next_state = SessionState::IDLE;
    //     state.has_event = true;
    // }
    // if state.session_state == SessionState::IDLE {
    //     next_state = SessionState::READY;
    //     state.has_event = true;
    // }
    // if state.session_state == SessionState::READY {
    //     next_state = SessionState::SYNCHRONIZED;
    //     state.has_event = true;
    // }
    // if state.session_state == SessionState::SYNCHRONIZED {
    //     next_state = SessionState::VISIBLE;
    //     state.has_event = true;
    // }
    // if state.session_state == SessionState::SYNCHRONIZED {
    //     next_state = SessionState::FOCUSED;
    //     state.has_event = true;
    // }

    // if state.has_event {
    //     let data = EventDataSessionStateChanged {
    //         ty: StructureType::EVENT_DATA_SESSION_STATE_CHANGED,
    //         next: ptr::null(),
    //         session: Session::from_raw(42),
    //         state: next_state,
    //         time: openxr_sys::Time::from_nanos(10),
    //     };
    //     copy_nonoverlapping(&data, transmute(event_data), 1);
    //     state.has_event = false;
    //     state.session_state = next_state;

    //     Result::ERROR_FEATURE_UNSUPPORTED
    // } else {
    Result::EVENT_UNAVAILABLE
    // }
}

pub unsafe extern "system" fn begin_session(
    session: Session,
    _begin_info: *const SessionBeginInfo,
) -> Result {
    debug!("[HOTHAM_SIMULATOR] Beginning session: {session:?}");
    Result::ERROR_FEATURE_UNSUPPORTED
}
pub unsafe extern "system" fn wait_frame(
    _session: Session,
    _frame_wait_info: *const FrameWaitInfo,
    frame_state: *mut FrameState,
) -> Result {
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn begin_frame(
    _session: Session,
    _frame_begin_info: *const FrameBeginInfo,
) -> Result {
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn enumerate_view_configuration_views(
    _instance: Instance,
    _system_id: SystemId,
    _view_configuration_type: ViewConfigurationType,
    view_capacity_input: u32,
    view_count_output: *mut u32,
    views: *mut ViewConfigurationView,
) -> Result {
    trace!("enumerate_view_configuration_views");
    let client = EDITOR_CLIENT.get_mut().unwrap();
    if view_capacity_input == 0 {
        let view_count = client.request(&requests::GetViewCount {}).unwrap();
        trace!("Received view count from server {view_count}");
        *view_count_output = view_count;
        return Result::SUCCESS;
    }

    let view_configuration = client.request(&requests::GetViewConfiguration {}).unwrap();

    set_array(
        view_capacity_input,
        view_count_output,
        views,
        [ViewConfigurationView {
            ty: StructureType::VIEW_CONFIGURATION_VIEW,
            next: null_mut(),
            recommended_image_rect_width: view_configuration.width,
            max_image_rect_height: view_configuration.height,
            recommended_swapchain_sample_count: 1,
            max_swapchain_sample_count: 1,
            max_image_rect_width: view_configuration.width,
            recommended_image_rect_height: view_configuration.height,
        }; 3],
    );

    Result::SUCCESS
}

pub unsafe extern "system" fn create_xr_swapchain(
    _session: Session,
    create_info: *const SwapchainCreateInfo,
    swapchain: *mut Swapchain,
) -> Result {
    trace!("create_swapchain");
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn enumerate_swapchain_images(
    _swapchain: Swapchain,
    image_capacity_input: u32,
    image_count_output: *mut u32,
    images: *mut SwapchainImageBaseHeader,
) -> Result {
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn acquire_swapchain_image(
    swapchain: Swapchain,
    _acquire_info: *const SwapchainImageAcquireInfo,
    index: *mut u32,
) -> Result {
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn wait_swapchain_image(
    _swapchain: Swapchain,
    _wait_info: *const SwapchainImageWaitInfo,
) -> Result {
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn dummy() -> Result {
    error!("[HOTHAM_OPENXR_CLIENT] Uh oh, dummy called");
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn locate_space(
    space: Space,
    _base_space: Space,
    _time: Time,
    location_out: *mut SpaceLocation,
) -> Result {
    // match STATE.lock().unwrap().spaces.get(&space.into_raw()) {
    //     Some(space_state) => {
    //         let pose = Posef {
    //             position: space_state.position,
    //             orientation: space_state.orientation,
    //         };
    //         *location_out = SpaceLocation {
    //             ty: StructureType::SPACE_LOCATION,
    //             next: null_mut(),
    //             location_flags: SpaceLocationFlags::ORIENTATION_TRACKED
    //                 | SpaceLocationFlags::POSITION_VALID
    //                 | SpaceLocationFlags::ORIENTATION_VALID,
    //             pose,
    //         };
    // Result::ERROR_FEATURE_UNSUPPORTED
    //     }
    //     None => Result::ERROR_HANDLE_INVALID,
    // }
    Result::ERROR_FEATURE_UNSUPPORTED
}
pub unsafe extern "system" fn get_action_state_pose(
    _session: Session,
    _get_info: *const ActionStateGetInfo,
    state: *mut ActionStatePose,
) -> Result {
    *state = ActionStatePose {
        ty: StructureType::ACTION_STATE_POSE,
        next: ptr::null_mut(),
        is_active: TRUE,
    };
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn sync_actions(
    _session: Session,
    _sync_info: *const ActionsSyncInfo,
) -> Result {
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn locate_views(
    _session: Session,
    _view_locate_info: *const ViewLocateInfo,
    view_state: *mut ViewState,
    view_capacity_input: u32,
    view_count_output: *mut u32,
    views: *mut View,
) -> Result {
    // *view_count_output = NUM_VIEWS as _;

    // if view_capacity_input == 0 {
    //     return Result::ERROR_FEATURE_UNSUPPORTED;
    // }

    // *view_state = ViewState {
    //     ty: StructureType::VIEW_STATE,
    //     next: null_mut(),
    //     view_state_flags: ViewStateFlags::ORIENTATION_VALID | ViewStateFlags::POSITION_VALID,
    // };
    // let views = slice::from_raw_parts_mut(views, NUM_VIEWS);
    // let state = STATE.lock().unwrap();
    // #[allow(clippy::approx_constant)]
    // for (i, view) in views.iter_mut().enumerate() {
    //     let pose = state.view_poses[i];

    //     // The actual fov is defined as (right - left). As these are all symetrical, we just divide the fov variable by 2.
    //     *view = View {
    //         ty: StructureType::VIEW,
    //         next: null_mut(),
    //         pose,
    //         fov: Fovf {
    //             angle_down: -CAMERA_FIELD_OF_VIEW / 2.,
    //             angle_up: CAMERA_FIELD_OF_VIEW / 2.,
    //             angle_left: -CAMERA_FIELD_OF_VIEW / 2.,
    //             angle_right: CAMERA_FIELD_OF_VIEW / 2.,
    //         },
    //     };
    // }

    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn release_swapchain_image(
    _swapchain: Swapchain,
    _release_info: *const SwapchainImageReleaseInfo,
) -> Result {
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn end_frame(
    _session: Session,
    frame_end_info: *const FrameEndInfo,
) -> Result {
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn request_exit_session(_session: Session) -> Result {
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn destroy_space(_space: Space) -> Result {
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn destroy_action(_action: Action) -> Result {
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn destroy_action_set(_action_set: ActionSet) -> Result {
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn destroy_swapchain(_swapchain: Swapchain) -> Result {
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn destroy_session(_session: Session) -> Result {
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn destroy_instance(_instance: Instance) -> Result {
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn enumerate_view_configurations(
    _instance: Instance,
    _system_id: SystemId,
    _view_configuration_type_capacity_input: u32,
    view_configuration_type_count_output: *mut u32,
    _view_configuration_types: *mut ViewConfigurationType,
) -> Result {
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn enumerate_reference_spaces(
    _session: Session,
    space_capacity_input: u32,
    space_count_output: *mut u32,
    spaces: *mut ReferenceSpaceType,
) -> Result {
    *space_count_output = 1;
    if space_capacity_input == 0 {
        return Result::ERROR_FEATURE_UNSUPPORTED;
    }

    let spaces = slice::from_raw_parts_mut(spaces, 1);
    spaces[0] = ReferenceSpaceType::STAGE;

    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn get_system_properties(
    _instance: Instance,
    _system_id: SystemId,
    _properties: *mut SystemProperties,
) -> Result {
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn enumerate_swapchain_formats(
    _session: Session,
    format_capacity_input: u32,
    format_count_output: *mut u32,
    formats: *mut i64,
) -> Result {
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn get_action_state_float(
    _session: Session,
    _get_info: *const ActionStateGetInfo,
    state: *mut ActionStateFloat,
) -> Result {
    *state = ActionStateFloat {
        ty: StructureType::ACTION_STATE_FLOAT,
        next: ptr::null_mut(),
        current_state: 0.0,
        changed_since_last_sync: FALSE,
        last_change_time: openxr_sys::Time::from_nanos(0),
        is_active: TRUE,
    };
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn end_session(_session: Session) -> Result {
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn get_action_state_boolean(
    _session: Session,
    get_info: *const ActionStateGetInfo,
    action_state: *mut ActionStateBoolean,
) -> Result {
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn apply_haptic_feedback(
    _session: Session,
    _haptic_action_info: *const HapticActionInfo,
    _haptic_feedback: *const HapticBaseHeader,
) -> Result {
    /* explicit no-op, could possibly be extended with controller support in future if winit ever
     * provides such APIs */
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn get_vulkan_instance_extensions(
    _instance: Instance,
    _system_id: SystemId,
    buffer_capacity_input: u32,
    buffer_count_output: *mut u32,
    buffer: *mut c_char,
) -> Result {
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn get_vulkan_device_extensions(
    _instance: Instance,
    _system_id: SystemId,
    buffer_capacity_input: u32,
    buffer_count_output: *mut u32,
    buffer: *mut c_char,
) -> Result {
    Result::ERROR_FEATURE_UNSUPPORTED
}

fn str_to_fixed_bytes(string: &str) -> [i8; 128] {
    let mut name = [0i8; 128];
    string
        .bytes()
        .zip(name.iter_mut())
        .for_each(|(b, ptr)| *ptr = b as i8);
    name
}

unsafe fn set_array<T: Copy, const COUNT: usize>(
    input_count: u32,
    output_count: *mut u32,
    array_ptr: *mut T,
    data: [T; COUNT],
) {
    if input_count == 0 {
        *output_count = data.len() as _;
        return;
    }

    // There's probably some clever way to do this without copying, but whatever
    let slice = slice::from_raw_parts_mut(array_ptr, COUNT);
    slice.copy_from_slice(&data);
}
