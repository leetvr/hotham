#![allow(
    non_snake_case,
    dead_code,
    non_upper_case_globals,
    non_camel_case_types
)]
use crate::{action_state::ActionState, space_state::SpaceState};
use ash::vk::{self, Handle};
use hotham_editor_protocol::{
    requests::{self, AcquireSwapchainImage, EndFrame, LocateView},
    responses::SwapchainInfo,
    EditorClient,
};
use log::{debug, error, trace};
use once_cell::sync::OnceCell;
use openxr_sys::{
    platform::{VkDevice, VkInstance, VkPhysicalDevice, VkResult},
    Action, ActionCreateInfo, ActionSet, ActionSetCreateInfo, ActionSpaceCreateInfo,
    ActionStateBoolean, ActionStateFloat, ActionStateGetInfo, ActionStatePose, ActionsSyncInfo,
    EnvironmentBlendMode, EventDataBuffer, EventDataSessionStateChanged, ExtensionProperties, Fovf,
    FrameBeginInfo, FrameEndInfo, FrameState, FrameWaitInfo, GraphicsBindingVulkanKHR,
    GraphicsRequirementsVulkanKHR, HapticActionInfo, HapticBaseHeader, Instance,
    InstanceCreateInfo, InstanceProperties, InteractionProfileSuggestedBinding, Path, Posef,
    Quaternionf, ReferenceSpaceCreateInfo, ReferenceSpaceType, Result, Session,
    SessionActionSetsAttachInfo, SessionBeginInfo, SessionCreateInfo, SessionState, Space,
    SpaceLocation, SpaceLocationFlags, StructureType, Swapchain, SwapchainCreateInfo,
    SwapchainImageAcquireInfo, SwapchainImageBaseHeader, SwapchainImageReleaseInfo,
    SwapchainImageVulkanKHR, SwapchainImageWaitInfo, SystemGetInfo, SystemId, SystemProperties,
    Time, Vector3f, Version, View, ViewConfigurationType, ViewConfigurationView, ViewLocateInfo,
    ViewState, ViewStateFlags, VulkanDeviceCreateInfoKHR, VulkanGraphicsDeviceGetInfoKHR,
    VulkanInstanceCreateInfoKHR, FALSE, TRUE,
};

use lazy_vulkan::vulkan_context::VulkanContext;
use std::{
    collections::HashMap,
    ffi::{c_char, CStr},
    mem::transmute,
    ptr::null_mut,
};
use std::{ptr, slice, time::Instant};
use uds_windows::UnixStream;

type PartialVulkan = (ash::Entry, ash::Instance);
type SpaceMap = HashMap<u64, SpaceState>;
type StringToPathMap = HashMap<String, Path>;
type PathToStringMap = HashMap<Path, String>;
type BindingMap = HashMap<Path, Action>;

// Used during the init phase
static mut PARTIAL_VULKAN: OnceCell<PartialVulkan> = OnceCell::new();
static INSTANCE: OnceCell<Instance> = OnceCell::new();
static SESSION: OnceCell<Session> = OnceCell::new();
static VULKAN_CONTEXT: OnceCell<VulkanContext> = OnceCell::new();
static mut SPACES: OnceCell<SpaceMap> = OnceCell::new();
static mut EDITOR_CLIENT: OnceCell<EditorClient<UnixStream>> = OnceCell::new();
static mut STRING_TO_PATH: OnceCell<StringToPathMap> = OnceCell::new();
static mut PATH_TO_STRING: OnceCell<PathToStringMap> = OnceCell::new();
static mut BINDINGS: OnceCell<BindingMap> = OnceCell::new();
static mut SWAPCHAIN_IMAGE_COUNT: u32 = 0; // handy to keep around
static mut SWAPCHAIN_IMAGES: OnceCell<Vec<vk::Image>> = OnceCell::new();
static mut SWAPCHAIN_SEMAPHORES: OnceCell<Vec<vk::Semaphore>> = OnceCell::new();
static mut SESSION_STATE: SessionState = SessionState::UNKNOWN;
static mut ACTION_STATE: OnceCell<ActionState> = OnceCell::new();
static CLOCK: OnceCell<Instant> = OnceCell::new();

// Camera, etc
pub const CAMERA_FIELD_OF_VIEW: f32 = 1.; // about 57 degrees

pub unsafe extern "system" fn enumerate_instance_extension_properties(
    _layer_names: *const ::std::os::raw::c_char,
    property_capacity_input: u32,
    property_count_output: *mut u32,
    properties: *mut ExtensionProperties,
) -> Result {
    trace!("enumerate_instance_extension_properties");

    set_array(
        property_capacity_input,
        property_count_output,
        properties,
        [ExtensionProperties {
            ty: StructureType::EXTENSION_PROPERTIES,
            next: ptr::null_mut(),
            extension_name: str_to_fixed_bytes("XR_KHR_vulkan_enable2"),
            extension_version: 1,
        }],
    );
    Result::SUCCESS
}

pub unsafe extern "system" fn create_instance(
    _create_info: *const InstanceCreateInfo,
    instance: *mut Instance,
) -> Result {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();
    trace!("create_instance");

    // Initialise our various maps
    let _ = SPACES.set(Default::default());
    let _ = STRING_TO_PATH.set(Default::default());
    let _ = PATH_TO_STRING.set(Default::default());
    let _ = BINDINGS.set(Default::default());
    let _ = ACTION_STATE.set(Default::default());

    // New instance, new luck.
    *instance = Instance::from_raw(rand::random());
    let _ = INSTANCE.set(*instance);

    // Mr. Gaeta, start the clock.
    CLOCK.set(Instant::now()).unwrap();

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
    *vulkan_instance = instance.handle().as_raw() as *const _;

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
    *vulkan_physical_device = physical_device.as_raw() as *const _;
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
        vk::PhysicalDevice::from_raw(create_info.vulkan_physical_device as u64);
    let device_create_info: &mut vk::DeviceCreateInfo =
        &mut *create_info.vulkan_create_info.cast_mut().cast(); // evil? probably
    let mut extension_names = std::slice::from_raw_parts(
        device_create_info.pp_enabled_extension_names,
        device_create_info.enabled_extension_count as _,
    )
    .to_vec();

    #[cfg(target_os = "windows")]
    extension_names.push(ash::extensions::khr::ExternalMemoryWin32::name().as_ptr());

    #[cfg(target_os = "windows")]
    extension_names.push(ash::extensions::khr::ExternalSemaphoreWin32::name().as_ptr());

    for e in &extension_names {
        let e = CStr::from_ptr(*e).to_str().unwrap();
        trace!("Application requested Vulkan extension: {e}")
    }

    device_create_info.enabled_extension_count = extension_names.len() as _;
    device_create_info.pp_enabled_extension_names = extension_names.as_ptr();

    trace!("Physical device: {physical_device:?}");
    trace!("Create info: {device_create_info:?}");

    let device = instance
        .create_device(physical_device, device_create_info, None)
        .unwrap();

    *vulkan_device = device.handle().as_raw() as *const _;

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
    _instance_properties: *mut InstanceProperties,
) -> Result {
    trace!("get_instance_properties");
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn enumerate_environment_blend_modes(
    _instance: Instance,
    _system_id: SystemId,
    _view_configuration_type: ViewConfigurationType,
    _environment_blend_mode_capacity_input: u32,
    _environment_blend_mode_count_output: *mut u32,
    _environment_blend_modes: *mut EnvironmentBlendMode,
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
    let physical_device = vk::PhysicalDevice::from_raw(graphics_binding.physical_device as u64);
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
    let name = CStr::from_ptr((*create_info).action_set_name.as_ptr());
    trace!("Creating action set with name {name:?}");
    *action_set = ActionSet::from_raw(rand::random());
    Result::SUCCESS
}

pub unsafe extern "system" fn create_action(
    _action_set: ActionSet,
    _create_info: *const ActionCreateInfo,
    action_out: *mut Action,
) -> Result {
    trace!("create_action");
    *action_out = Action::from_raw(rand::random());
    Result::SUCCESS
}

pub unsafe extern "system" fn suggest_interaction_profile_bindings(
    _instance: Instance,
    suggested_bindings: *const InteractionProfileSuggestedBinding,
) -> Result {
    trace!("suggest_interaction_profile_bindings");
    let suggested_bindings = *suggested_bindings;

    let bindings = std::slice::from_raw_parts(
        suggested_bindings.suggested_bindings,
        suggested_bindings.count_suggested_bindings as _,
    );

    let bindings_map = BINDINGS.get_mut().unwrap();

    for binding in bindings {
        bindings_map.insert(binding.binding, binding.action);
    }

    Result::SUCCESS
}

pub unsafe extern "system" fn string_to_path(
    _instance: Instance,
    path_string: *const c_char,
    path_out: *mut Path,
) -> Result {
    trace!("string_to_path");
    match CStr::from_ptr(path_string).to_str() {
        Ok(path_string) => {
            let path = Path::from_raw(rand::random());
            trace!("Adding ({path_string}, {path:?}) to path map");
            STRING_TO_PATH
                .get_mut()
                .unwrap()
                .insert(path_string.to_string(), path);
            PATH_TO_STRING
                .get_mut()
                .unwrap()
                .insert(path, path_string.to_string());
            *path_out = path;
            Result::SUCCESS
        }
        Err(_) => Result::ERROR_VALIDATION_FAILURE,
    }
}

pub unsafe extern "system" fn attach_action_sets(
    _session: Session,
    _attach_info: *const SessionActionSetsAttachInfo,
) -> Result {
    trace!("attach_action_sets");
    Result::SUCCESS
}

// TODO: Handle aim pose.
pub unsafe extern "system" fn create_action_space(
    _session: Session,
    create_info: *const ActionSpaceCreateInfo,
    space_out: *mut Space,
) -> Result {
    trace!("create_action_space");
    let path_string = PATH_TO_STRING
        .get()
        .unwrap()
        .get(&(*create_info).subaction_path)
        .map(|s| s.as_str());
    let space = Space::from_raw(rand::random());
    let spaces = SPACES.get_mut().unwrap();

    match path_string {
        Some("/user/hand/left") => {
            let mut space_state = SpaceState::new("Left Hand");
            space_state.position = Vector3f {
                x: -0.20,
                y: 1.4,
                z: -0.50,
            };
            space_state.orientation = Quaternionf {
                x: 0.707,
                y: 0.,
                z: 0.,
                w: 0.707,
            };
            trace!("Created left hand space: {space_state:?}, {space:?}");
            spaces.insert(space.into_raw(), space_state);
        }
        Some("/user/hand/right") => {
            let mut space_state = SpaceState::new("Right Hand");
            space_state.orientation = Quaternionf {
                x: 0.707,
                y: 0.,
                z: 0.,
                w: 0.707,
            };
            space_state.position = Vector3f {
                x: 0.20,
                y: 1.4,
                z: -0.50,
            };
            trace!("Created right hand space: {space_state:?}, {space:?}");
            spaces.insert(space.into_raw(), space_state);
        }
        Some(path) => {
            let space_state = SpaceState::new(path);
            trace!("Created new space: {space_state:?}, {space:?}");
            spaces.insert(space.into_raw(), space_state);
        }
        _ => return Result::ERROR_PATH_INVALID,
    };

    *space_out = space;
    Result::SUCCESS
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
    let next_state = match SESSION_STATE {
        SessionState::UNKNOWN => Some(SessionState::IDLE),
        SessionState::IDLE => Some(SessionState::READY),
        SessionState::READY => Some(SessionState::SYNCHRONIZED),
        SessionState::SYNCHRONIZED => Some(SessionState::VISIBLE),
        SessionState::VISIBLE => Some(SessionState::FOCUSED),
        _ => None,
    };

    if let Some(next_state) = next_state {
        SESSION_STATE = next_state;
        let event_data = event_data as *mut EventDataSessionStateChanged;
        *event_data = EventDataSessionStateChanged {
            ty: StructureType::EVENT_DATA_SESSION_STATE_CHANGED,
            next: ptr::null(),
            session: *SESSION.get().unwrap(),
            state: next_state,
            time: now(),
        };
        return Result::SUCCESS;
    }

    Result::EVENT_UNAVAILABLE
}

pub unsafe extern "system" fn begin_session(
    session: Session,
    _begin_info: *const SessionBeginInfo,
) -> Result {
    trace!("begin_session");
    debug!("Beginning session: {session:?}");
    Result::SUCCESS
}
pub unsafe extern "system" fn wait_frame(
    _session: Session,
    _frame_wait_info: *const FrameWaitInfo,
    frame_state: *mut FrameState,
) -> Result {
    trace!("wait_frame");

    // This is a bit of a hack, but if we're not in the FOCUSED state, we'll be sending `wait_frame` before
    // `locate_views` which will annoy the editor.
    if SESSION_STATE != SessionState::FOCUSED {
        *frame_state = FrameState {
            ty: StructureType::FRAME_STATE,
            next: null_mut(),
            predicted_display_time: now(),
            predicted_display_period: openxr_sys::Duration::from_nanos(1),
            should_render: false.into(),
        };

        return Result::SUCCESS;
    }

    let client = EDITOR_CLIENT.get_mut().unwrap();
    client.request(&requests::WaitFrame).unwrap();

    *frame_state = FrameState {
        ty: StructureType::FRAME_STATE,
        next: null_mut(),
        predicted_display_time: now(),
        predicted_display_period: openxr_sys::Duration::from_nanos(1),
        should_render: true.into(),
    };

    Result::SUCCESS
}

pub unsafe extern "system" fn begin_frame(
    _session: Session,
    _frame_begin_info: *const FrameBeginInfo,
) -> Result {
    trace!("begin_frame");
    Result::SUCCESS
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
        SWAPCHAIN_IMAGE_COUNT = view_count;
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
    _create_info: *const SwapchainCreateInfo,
    swapchain: *mut Swapchain,
) -> Result {
    trace!("create_swapchain");
    *swapchain = Swapchain::from_raw(rand::random());
    Result::SUCCESS
}

pub unsafe extern "system" fn enumerate_swapchain_images(
    _swapchain: Swapchain,
    image_capacity_input: u32,
    image_count_output: *mut u32,
    images: *mut SwapchainImageBaseHeader,
) -> Result {
    trace!("enumerate_swapchain_images");
    if image_capacity_input == 0 {
        *image_count_output = SWAPCHAIN_IMAGE_COUNT;
        return Result::SUCCESS;
    }

    let client = EDITOR_CLIENT.get_mut().unwrap();

    trace!("Requesting swapchain info");
    let swapchain_info = client.request(&requests::GetSwapchainInfo {}).unwrap();
    trace!("Got swapchain info {swapchain_info:?}");

    trace!("Requesting swapchain image handles..");
    let swapchain_image_handles = client
        .request_vec(&requests::GetSwapchainImages {})
        .unwrap();
    trace!("Got swapchain image handles {swapchain_image_handles:?}");

    trace!("Requesting semaphore handles..");
    let semaphore_handles = client
        .request_vec(&requests::GetSwapchainSemaphores {})
        .unwrap();
    trace!("Got semaphore handles {semaphore_handles:?}");

    let swapchain_images = create_swapchain_images(swapchain_image_handles, swapchain_info);
    trace!("Created swapchain images {swapchain_images:?}");

    let _ = SWAPCHAIN_SEMAPHORES.set(create_swapchain_semaphores(semaphore_handles));
    let _ = SWAPCHAIN_IMAGES.set(swapchain_images.clone());

    let output_images = std::slice::from_raw_parts_mut(
        images as *mut SwapchainImageVulkanKHR,
        SWAPCHAIN_IMAGE_COUNT as _,
    );
    for (output_image, swapchain_image) in output_images.iter_mut().zip(swapchain_images.iter()) {
        *output_image = SwapchainImageVulkanKHR {
            ty: StructureType::SWAPCHAIN_IMAGE_VULKAN_KHR,
            next: null_mut(),
            image: swapchain_image.as_raw(),
        };
    }

    Result::SUCCESS
}

pub unsafe extern "system" fn acquire_swapchain_image(
    _swapchain: Swapchain,
    _acquire_info: *const SwapchainImageAcquireInfo,
    index: *mut u32,
) -> Result {
    trace!("acquire_swapchain_image");
    // This is a bit of a hack, but if we're not in the FOCUSED state, we'll be sending `acquire_swapchain_image` before
    // `locate_views` which will annoy the editor.
    if SESSION_STATE != SessionState::FOCUSED {
        *index = 0;
        return Result::SUCCESS;
    }

    let client = EDITOR_CLIENT.get_mut().unwrap();
    *index = client.request(&AcquireSwapchainImage).unwrap();
    Result::SUCCESS
}

pub unsafe extern "system" fn wait_swapchain_image(
    _swapchain: Swapchain,
    _wait_info: *const SwapchainImageWaitInfo,
) -> Result {
    trace!("wait_swapchain_image");
    Result::SUCCESS
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
    trace!("locate_space");
    match SPACES.get().unwrap().get(&space.into_raw()) {
        Some(space_state) => {
            let pose = Posef {
                position: space_state.position,
                orientation: space_state.orientation,
            };
            *location_out = SpaceLocation {
                ty: StructureType::SPACE_LOCATION,
                next: null_mut(),
                location_flags: SpaceLocationFlags::ORIENTATION_TRACKED
                    | SpaceLocationFlags::POSITION_VALID
                    | SpaceLocationFlags::ORIENTATION_VALID,
                pose,
            };
            Result::SUCCESS
        }
        None => Result::ERROR_HANDLE_INVALID,
    }
}
pub unsafe extern "system" fn get_action_state_pose(
    _session: Session,
    _get_info: *const ActionStateGetInfo,
    state: *mut ActionStatePose,
) -> Result {
    trace!("get_action_state_pose");
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
    trace!("sync_actions");
    Result::SUCCESS
}

pub unsafe extern "system" fn locate_views(
    _session: Session,
    _view_locate_info: *const ViewLocateInfo,
    view_state: *mut ViewState,
    view_capacity_input: u32,
    view_count_output: *mut u32,
    views: *mut View,
) -> Result {
    trace!("locate_views");

    // To avoid hitting the editor twice, early return
    if view_capacity_input == 0 {
        *view_count_output = 2;
        return Result::SUCCESS;
    }

    let editor_client = EDITOR_CLIENT.get_mut().unwrap();
    let pose = editor_client.request(&LocateView).unwrap();

    let view = View {
        ty: StructureType::VIEW,
        next: null_mut(),
        pose,
        fov: Fovf {
            angle_down: -CAMERA_FIELD_OF_VIEW / 2.,
            angle_up: CAMERA_FIELD_OF_VIEW / 2.,
            angle_left: -CAMERA_FIELD_OF_VIEW / 2.,
            angle_right: CAMERA_FIELD_OF_VIEW / 2.,
        },
    };
    set_array(view_capacity_input, view_count_output, views, [view; 2]);
    *view_state = ViewState {
        ty: StructureType::VIEW_STATE,
        next: null_mut(),
        view_state_flags: ViewStateFlags::ORIENTATION_VALID | ViewStateFlags::POSITION_VALID,
    };

    Result::SUCCESS
}

pub unsafe extern "system" fn release_swapchain_image(
    _swapchain: Swapchain,
    _release_info: *const SwapchainImageReleaseInfo,
) -> Result {
    trace!("release_swapchain_images");
    Result::SUCCESS
}

pub unsafe extern "system" fn end_frame(
    _session: Session,
    _frame_end_info: *const FrameEndInfo,
) -> Result {
    trace!("end_frame");
    EDITOR_CLIENT.get_mut().unwrap().request(&EndFrame).unwrap();
    Result::SUCCESS
}

pub unsafe extern "system" fn request_exit_session(_session: Session) -> Result {
    trace!("request_exit_session");
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn destroy_space(_space: Space) -> Result {
    trace!("destroy_space");
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn destroy_action(_action: Action) -> Result {
    trace!("destroy_actions");
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn destroy_action_set(_action_set: ActionSet) -> Result {
    trace!("destroy_action_set");
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn destroy_swapchain(_swapchain: Swapchain) -> Result {
    trace!("destroy_swapchain");
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn destroy_session(_session: Session) -> Result {
    trace!("destroy_session");
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn destroy_instance(_instance: Instance) -> Result {
    trace!("destroy_instance");
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn enumerate_view_configurations(
    _instance: Instance,
    _system_id: SystemId,
    _view_configuration_type_capacity_input: u32,
    _view_configuration_type_count_output: *mut u32,
    _view_configuration_types: *mut ViewConfigurationType,
) -> Result {
    trace!("enumerate_view_configurations");
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn enumerate_reference_spaces(
    _session: Session,
    space_capacity_input: u32,
    space_count_output: *mut u32,
    spaces: *mut ReferenceSpaceType,
) -> Result {
    trace!("enumerate_reference_spaces");
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
    trace!("get_system_properties");
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn enumerate_swapchain_formats(
    _session: Session,
    _format_capacity_input: u32,
    _format_count_output: *mut u32,
    _formats: *mut i64,
) -> Result {
    trace!("enumerate_swapchain_formats");
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn get_action_state_float(
    _session: Session,
    _get_info: *const ActionStateGetInfo,
    state: *mut ActionStateFloat,
) -> Result {
    trace!("get_action_state_float");
    *state = ActionStateFloat {
        ty: StructureType::ACTION_STATE_FLOAT,
        next: ptr::null_mut(),
        current_state: 0.0,
        changed_since_last_sync: FALSE,
        last_change_time: openxr_sys::Time::from_nanos(0),
        is_active: TRUE,
    };
    Result::SUCCESS
}

pub unsafe extern "system" fn end_session(_session: Session) -> Result {
    trace!("end_session");
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn get_action_state_boolean(
    _session: Session,
    get_info: *const ActionStateGetInfo,
    action_state: *mut ActionStateBoolean,
) -> Result {
    trace!("get_action_state_boolean");
    let current_state = ACTION_STATE.get().unwrap().get_boolean((*get_info).action);
    *action_state = ActionStateBoolean {
        ty: StructureType::ACTION_STATE_BOOLEAN,
        next: ptr::null_mut(),
        current_state,
        changed_since_last_sync: FALSE,
        last_change_time: openxr_sys::Time::from_nanos(0),
        is_active: TRUE,
    };
    Result::SUCCESS
}

pub unsafe extern "system" fn apply_haptic_feedback(
    _session: Session,
    _haptic_action_info: *const HapticActionInfo,
    _haptic_feedback: *const HapticBaseHeader,
) -> Result {
    trace!("apply_haptic_feedback");
    /* explicit no-op, could possibly be extended with controller support in future if winit ever
     * provides such APIs */
    Result::SUCCESS
}

pub unsafe extern "system" fn get_vulkan_instance_extensions(
    _instance: Instance,
    _system_id: SystemId,
    _buffer_capacity_input: u32,
    _buffer_count_output: *mut u32,
    _buffer: *mut c_char,
) -> Result {
    trace!("get_vulkan_instance_extensions");
    Result::ERROR_FEATURE_UNSUPPORTED
}

pub unsafe extern "system" fn get_vulkan_device_extensions(
    _instance: Instance,
    _system_id: SystemId,
    _buffer_capacity_input: u32,
    _buffer_count_output: *mut u32,
    _buffer: *mut c_char,
) -> Result {
    trace!("get_vulkan_device_extensions");
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

fn create_swapchain_images(
    handles: Vec<vk::HANDLE>,
    swapchain_info: SwapchainInfo,
) -> Vec<vk::Image> {
    let vulkan_context = VULKAN_CONTEXT.get().unwrap();
    let device = &vulkan_context.device;

    handles
        .into_iter()
        .map(|handle| unsafe {
            trace!("Creating image..");
            let handle_type = vk::ExternalMemoryHandleTypeFlags::OPAQUE_WIN32_KMT;

            let mut external_memory_image_create_info =
                vk::ExternalMemoryImageCreateInfo::builder().handle_types(handle_type);
            let image = device
                .create_image(
                    &vk::ImageCreateInfo {
                        image_type: vk::ImageType::TYPE_2D,
                        format: swapchain_info.format,
                        extent: swapchain_info.resolution.into(),
                        mip_levels: 1,
                        array_layers: 2,
                        samples: vk::SampleCountFlags::TYPE_1,
                        tiling: vk::ImageTiling::OPTIMAL,
                        usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
                        sharing_mode: vk::SharingMode::EXCLUSIVE,
                        p_next: &mut external_memory_image_create_info as *mut _ as *mut _,
                        ..Default::default()
                    },
                    None,
                )
                .unwrap();
            trace!("Allocating image memory..");
            let requirements = device.get_image_memory_requirements(image);
            let mut external_memory_allocate_info = vk::ImportMemoryWin32HandleInfoKHR::builder()
                .handle(handle)
                .handle_type(handle_type);
            let memory = device
                .allocate_memory(
                    &vk::MemoryAllocateInfo::builder()
                        .allocation_size(requirements.size)
                        .push_next(&mut external_memory_allocate_info),
                    None,
                )
                .unwrap();
            trace!("Done, allocating..");
            device.bind_image_memory(image, memory, 0).unwrap();
            image
        })
        .collect()
}

fn create_swapchain_semaphores(handles: Vec<vk::HANDLE>) -> Vec<vk::Semaphore> {
    let vulkan_context = VULKAN_CONTEXT.get().unwrap();
    let device = &vulkan_context.device;
    let external_semaphore = ash::extensions::khr::ExternalSemaphoreWin32::new(
        &vulkan_context.instance,
        &vulkan_context.device,
    );
    let handle_type = vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_WIN32_KMT;

    handles
        .iter()
        .map(|h| unsafe {
            let mut external_semaphore_info =
                vk::ExportSemaphoreCreateInfo::builder().handle_types(handle_type);
            let semaphore = device
                .create_semaphore(
                    &vk::SemaphoreCreateInfo::builder().push_next(&mut external_semaphore_info),
                    None,
                )
                .unwrap();

            external_semaphore
                .import_semaphore_win32_handle(
                    &vk::ImportSemaphoreWin32HandleInfoKHR::builder()
                        .handle(*h)
                        .semaphore(semaphore)
                        .handle_type(handle_type),
                )
                .unwrap();

            semaphore
        })
        .collect()
}

fn now() -> openxr_sys::Time {
    openxr_sys::Time::from_nanos(
        (std::time::Instant::now() - *CLOCK.get().unwrap()).as_nanos() as _,
    )
}
