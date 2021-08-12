use anyhow::{anyhow, Result};
use ash::{
    version::InstanceV1_0,
    vk::{self, Handle},
};
use openxr::{
    self as xr, Action, ActionSet, EventDataBuffer, FrameStream, FrameWaiter, Path, Posef, Session,
    SessionState, Space, Swapchain, VulkanLegacy,
};
use xr::{
    vulkan_legacy::SessionCreateInfo, FrameState, ReferenceSpaceType, SwapchainCreateFlags,
    SwapchainCreateInfo, SwapchainUsageFlags, View,
};

use crate::{resources::VulkanContext, COLOR_FORMAT, VIEW_COUNT, VIEW_TYPE};

pub struct XrContext {
    pub instance: openxr::Instance,
    pub session: Session<VulkanLegacy>,
    pub session_state: SessionState,
    pub swapchain: Swapchain<VulkanLegacy>,
    pub reference_space: Space,
    pub action_set: ActionSet,
    pub pose_action: Action<Posef>,
    pub grab_action: Action<f32>,
    pub left_hand_space: Space,
    pub left_hand_subaction_path: Path,
    pub right_hand_space: Space,
    pub right_hand_subaction_path: Path,
    pub swapchain_resolution: vk::Extent2D,
    pub event_buffer: EventDataBuffer,
    pub frame_waiter: FrameWaiter,
    pub frame_stream: FrameStream<VulkanLegacy>,
    pub frame_state: Option<FrameState>,
    pub views: Vec<View>,
}

impl XrContext {
    pub(crate) fn new() -> Result<(XrContext, VulkanContext)> {
        let (instance, system) = create_xr_instance()?;
        let vulkan_context = create_vulkan_context(&instance, system)?;
        let (session, frame_waiter, frame_stream) =
            create_xr_session(&instance, system, &vulkan_context)?; // TODO: Extract to XRContext
        let reference_space =
            session.create_reference_space(ReferenceSpaceType::STAGE, xr::Posef::IDENTITY)?;
        let swapchain_resolution = get_swapchain_resolution(&instance, system)?;
        let swapchain = create_xr_swapchain(&session, &swapchain_resolution, VIEW_COUNT)?;

        // Create an action set to encapsulate our actions
        let action_set = instance.create_action_set("input", "input pose information", 0)?;

        let left_hand_subaction_path = instance.string_to_path("/user/hand/left").unwrap();
        let right_hand_subaction_path = instance.string_to_path("/user/hand/right").unwrap();
        let left_hand_pose_path = instance
            .string_to_path("/user/hand/left/input/grip/pose")
            .unwrap();
        let right_hand_pose_path = instance
            .string_to_path("/user/hand/right/input/grip/pose")
            .unwrap();

        let left_hand_grip_squeeze_path = instance
            .string_to_path("/user/hand/left/input/squeeze/value")
            .unwrap();
        let right_hand_grip_squeeze_path = instance
            .string_to_path("/user/hand/right/input/squeeze/value")
            .unwrap();

        let pose_action = action_set.create_action::<xr::Posef>(
            "hand_pose",
            "Hand Pose",
            &[left_hand_subaction_path, right_hand_subaction_path],
        )?;

        let grab_action = action_set.create_action::<f32>(
            "grab_object",
            "Grab Object",
            &[left_hand_subaction_path, right_hand_subaction_path],
        )?;

        // Bind our actions to input devices using the given profile
        instance.suggest_interaction_profile_bindings(
            instance
                .string_to_path("/interaction_profiles/oculus/touch_controller")
                .unwrap(),
            &[
                xr::Binding::new(&pose_action, left_hand_pose_path),
                xr::Binding::new(&pose_action, right_hand_pose_path),
                xr::Binding::new(&grab_action, left_hand_grip_squeeze_path),
                xr::Binding::new(&grab_action, right_hand_grip_squeeze_path),
            ],
        )?;

        let left_hand_space =
            pose_action.create_space(session.clone(), left_hand_subaction_path, Posef::IDENTITY)?;
        let right_hand_space = pose_action.create_space(
            session.clone(),
            right_hand_subaction_path,
            Posef::IDENTITY,
        )?;

        // Attach the action set to the session
        session.attach_action_sets(&[&action_set])?;
        let xr_context = XrContext {
            instance,
            session,
            session_state: SessionState::IDLE,
            swapchain,
            reference_space,
            action_set,
            pose_action,
            grab_action,
            left_hand_space,
            left_hand_subaction_path,
            right_hand_space,
            right_hand_subaction_path,
            swapchain_resolution,
            event_buffer: Default::default(),
            frame_waiter,
            frame_stream,
            frame_state: None,
            views: Vec::new(),
        };

        Ok((xr_context, vulkan_context))
    }

    pub(crate) fn poll_xr_event(&mut self) -> Result<SessionState> {
        loop {
            match self.instance.poll_event(&mut self.event_buffer)? {
                Some(xr::Event::SessionStateChanged(session_changed)) => {
                    let new_state = session_changed.state();

                    if self.session_state == SessionState::IDLE && new_state == SessionState::READY
                    {
                        println!("[HOTHAM_POLL_EVENT] Beginning session!");
                        self.session.begin(VIEW_TYPE)?;
                    }

                    if self.session_state != SessionState::STOPPING
                        && new_state == SessionState::STOPPING
                    {
                        println!("[HOTHAM_POLL_EVENT] Ending session!");
                        self.session.end()?;
                    }

                    println!("[HOTHAM_POLL_EVENT] State is now {:?}", new_state);
                    self.session_state = new_state;
                }
                Some(_) => {
                    println!("[HOTHAM_POLL_EVENT] Received some other event");
                }
                None => break,
            }
        }

        Ok(self.session_state)
    }

    pub(crate) fn begin_frame(&mut self) -> Result<(xr::FrameState, usize)> {
        let frame_state = self.frame_waiter.wait()?;
        self.frame_stream.begin()?;

        let image_index = self.swapchain.acquire_image()?;
        self.swapchain.wait_image(openxr::Duration::INFINITE)?;

        Ok((frame_state, image_index as _))
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn create_vulkan_context(
    xr_instance: &xr::Instance,
    system: xr::SystemId,
) -> Result<VulkanContext, crate::hotham_error::HothamError> {
    let vulkan_context = VulkanContext::create_from_xr_instance_legacy(xr_instance, system)?;
    println!("[HOTHAM_VULKAN] - Vulkan Context created successfully");
    Ok(vulkan_context)
}

#[cfg(target_os = "android")]
fn create_vulkan_context(
    xr_instance: &xr::Instance,
    system: xr::SystemId,
) -> Result<VulkanContext, crate::hotham_error::HothamError> {
    let vulkan_context = VulkanContext::create_from_xr_instance_legacy(xr_instance, system)?;
    println!("[HOTHAM_VULKAN] - Vulkan Context created successfully");
    Ok(vulkan_context)
}

pub(crate) fn get_swapchain_resolution(
    xr_instance: &xr::Instance,
    system: xr::SystemId,
) -> Result<vk::Extent2D> {
    let views = xr_instance.enumerate_view_configuration_views(system, VIEW_TYPE)?;
    println!("[HOTHAM_VULKAN] Views: {:?}", views);
    let resolution = vk::Extent2D {
        width: views[0].recommended_image_rect_width,
        height: views[0].recommended_image_rect_height,
    };

    Ok(resolution)
}

pub(crate) fn create_xr_swapchain(
    xr_session: &Session<VulkanLegacy>,
    resolution: &vk::Extent2D,
    array_size: u32,
) -> Result<Swapchain<VulkanLegacy>> {
    xr_session
        .create_swapchain(&SwapchainCreateInfo {
            create_flags: SwapchainCreateFlags::EMPTY,
            usage_flags: SwapchainUsageFlags::COLOR_ATTACHMENT,
            format: COLOR_FORMAT.as_raw() as u32,
            sample_count: 1,
            width: resolution.width,
            height: resolution.height,
            face_count: 1,
            array_size,
            mip_count: 1,
        })
        .map_err(Into::into)
}

pub(crate) fn create_xr_session(
    xr_instance: &xr::Instance,
    system: xr::SystemId,
    vulkan_context: &VulkanContext,
) -> Result<(
    Session<VulkanLegacy>,
    FrameWaiter,
    FrameStream<VulkanLegacy>,
)> {
    println!("[HOTHAM] Creating session..");
    Ok(unsafe {
        xr_instance.create_session(
            system,
            &SessionCreateInfo {
                instance: vulkan_context.instance.handle().as_raw() as *const _,
                physical_device: vulkan_context.physical_device.as_raw() as *const _,
                device: vulkan_context.device.handle().as_raw() as *const _,
                queue_family_index: vulkan_context.queue_family_index,
                queue_index: 0,
            },
        )
    }
    .unwrap())
}

#[cfg(not(target_os = "android"))]
pub(crate) fn create_xr_instance() -> anyhow::Result<(xr::Instance, xr::SystemId)> {
    let xr_entry = xr::Entry::load()?;
    let xr_app_info = openxr::ApplicationInfo {
        application_name: "Hotham Asteroid",
        application_version: 1,
        engine_name: "Hotham",
        engine_version: 1,
    };
    println!(
        "Available extensions: {:?}",
        xr_entry.enumerate_extensions()?
    );
    let mut required_extensions = xr::ExtensionSet::default();
    // required_extensions.khr_vulkan_enable2 = true; // TODO: Should we use enable 2 for the simulator..?
    required_extensions.khr_vulkan_enable = true; // TODO: Should we use enable 2 for the simulator..?
    let instance = xr_entry.create_instance(&xr_app_info, &required_extensions, &[])?;
    let system = instance.system(xr::FormFactor::HEAD_MOUNTED_DISPLAY)?;
    Ok((instance, system))
}

#[cfg(target_os = "android")]
fn create_xr_instance() -> anyhow::Result<(xr::Instance, xr::SystemId)> {
    use openxr::sys::{InstanceCreateInfoAndroidKHR, LoaderInitInfoAndroidKHR};
    use std::{ffi::CStr, intrinsics::transmute, ptr::null};
    use xr::sys::pfn::InitializeLoaderKHR;

    let xr_entry = xr::Entry::load()?;
    let native_activity = ndk_glue::native_activity();
    let vm_ptr = native_activity.vm();
    let context = native_activity.activity();

    unsafe {
        let mut initialize_loader = None;
        let get_instance_proc_addr = xr_entry.fp().get_instance_proc_addr;
        println!("[HOTHAM_ANDROID] About to call get_instance_proc_addr..");

        get_instance_proc_addr(
            Default::default(),
            CStr::from_bytes_with_nul_unchecked(b"xrInitializeLoaderKHR\0").as_ptr(),
            &mut initialize_loader,
        );

        let initialize_loader = initialize_loader.ok_or(anyhow!(
            "Couldn't get function pointer for xrInitialiseLoaderKHR"
        ))?;
        let initialize_loader: InitializeLoaderKHR = transmute(initialize_loader);

        let loader_init_info = LoaderInitInfoAndroidKHR {
            ty: LoaderInitInfoAndroidKHR::TYPE,
            next: null(),
            application_vm: vm_ptr as _,
            application_context: context as _,
        };

        println!(
            "[HOTHAM_ANDROID] Done! Calling loader init info with: {:?}",
            loader_init_info.ty
        );
        initialize_loader(transmute(&loader_init_info));
        println!("[HOTHAM_ANDROID] Done! Loader initialized.");
    }

    let extensions = xr_entry.enumerate_extensions();
    println!("[HOTHAM_ANDROID] Available extensions: {:?}", extensions);
    let layers = xr_entry.enumerate_layers();
    println!("[HOTHAM_ANDROID] Available layers: {:?}", layers);

    let xr_app_info = openxr::ApplicationInfo {
        application_name: "Hotham Asteroid",
        application_version: 1,
        engine_name: "Hotham",
        engine_version: 1,
    };
    let mut required_extensions = xr::ExtensionSet::default();
    required_extensions.ext_debug_utils = true;
    required_extensions.khr_vulkan_enable = true;
    required_extensions.khr_android_create_instance = true;
    print!("[HOTHAM_ANDROID] Creating instance..");
    let instance_create_info_android = InstanceCreateInfoAndroidKHR {
        ty: InstanceCreateInfoAndroidKHR::TYPE,
        next: null(),
        application_vm: vm_ptr as _,
        application_activity: context as _,
    };

    let instance = xr_entry.create_instance_android(
        &xr_app_info,
        &required_extensions,
        &[],
        &instance_create_info_android,
    )?;

    let system = instance.system(xr::FormFactor::HEAD_MOUNTED_DISPLAY)?;
    println!(" ..done!");
    Ok((instance, system))
}

#[cfg(test)]
mod tests {
    use super::XrContext;

    #[test]
    pub fn test_xr_context_smoke_test() {
        XrContext::new().unwrap();
    }
}
