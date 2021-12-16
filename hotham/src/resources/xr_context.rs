use anyhow::Result;
use ash::vk::{self, Handle};
use openxr::{
    self as xr, Action, ActionSet, EventDataBuffer, FrameStream, FrameWaiter, Path, Posef, Session,
    SessionState, Space, Swapchain, Vulkan,
};
use xr::{
    vulkan::SessionCreateInfo, Duration, FrameState, ReferenceSpaceType, SwapchainCreateFlags,
    SwapchainCreateInfo, SwapchainUsageFlags, Time, View,
};

use crate::{resources::VulkanContext, BLEND_MODE, COLOR_FORMAT, VIEW_COUNT, VIEW_TYPE};

pub struct XrContext {
    pub instance: openxr::Instance,
    pub session: Session<Vulkan>,
    pub session_state: SessionState,
    pub swapchain: Swapchain<Vulkan>,
    pub reference_space: Space,
    pub action_set: ActionSet,
    pub pose_action: Action<Posef>,
    pub grab_action: Action<f32>,
    pub left_hand_space: Space,
    pub left_hand_subaction_path: Path,
    pub right_hand_space: Space,
    pub right_hand_subaction_path: Path,
    pub swapchain_resolution: vk::Extent2D,
    pub frame_waiter: FrameWaiter,
    pub frame_stream: FrameStream<Vulkan>,
    pub frame_state: FrameState,
    pub views: Vec<View>,
}

impl XrContext {
    pub fn new() -> Result<(XrContext, VulkanContext)> {
        let (instance, system) = create_xr_instance()?;
        XrContext::_new(instance, system)
    }

    pub fn new_from_path(path: &std::path::Path) -> Result<(XrContext, VulkanContext)> {
        let (instance, system) = create_xr_instance_from_path(path)?;
        XrContext::_new(instance, system)
    }

    fn _new(instance: xr::Instance, system: xr::SystemId) -> Result<(XrContext, VulkanContext)> {
        let vulkan_context = create_vulkan_context(&instance, system)?;
        let (session, frame_waiter, frame_stream) =
            create_xr_session(&instance, system, &vulkan_context)?;
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

        let frame_state = FrameState {
            predicted_display_time: Time::from_nanos(0),
            predicted_display_period: Duration::from_nanos(0),
            should_render: false,
        };

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
            frame_waiter,
            frame_stream,
            frame_state,
            views: Vec::new(),
        };

        Ok((xr_context, vulkan_context))
    }

    pub(crate) fn poll_xr_event(
        &mut self,
        event_buffer: &mut EventDataBuffer,
    ) -> Result<SessionState> {
        loop {
            match self.instance.poll_event(event_buffer)? {
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

    pub fn end_frame(&mut self) -> std::result::Result<(), openxr::sys::Result> {
        // Submit the image to OpenXR
        self.swapchain.release_image().unwrap();

        let rect = xr::Rect2Di {
            offset: xr::Offset2Di { x: 0, y: 0 },
            extent: xr::Extent2Di {
                width: self.swapchain_resolution.width as _,
                height: self.swapchain_resolution.height as _,
            },
        };

        let display_time = self.frame_state.predicted_display_time;

        let views = [
            xr::CompositionLayerProjectionView::new()
                .pose(self.views[0].pose)
                .fov(self.views[0].fov)
                .sub_image(
                    xr::SwapchainSubImage::new()
                        .swapchain(&self.swapchain)
                        .image_array_index(0)
                        .image_rect(rect),
                ),
            xr::CompositionLayerProjectionView::new()
                .pose(self.views[1].pose)
                .fov(self.views[1].fov)
                .sub_image(
                    xr::SwapchainSubImage::new()
                        .swapchain(&self.swapchain)
                        .image_array_index(1)
                        .image_rect(rect),
                ),
        ];

        let layer_projection = xr::CompositionLayerProjection::new()
            .space(&self.reference_space)
            .views(&views);

        let layers = [&*layer_projection];
        self.frame_stream.end(display_time, BLEND_MODE, &layers)
    }
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
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
    xr_session: &Session<Vulkan>,
    resolution: &vk::Extent2D,
    array_size: u32,
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
            array_size,
            mip_count: 1,
        })
        .map_err(Into::into)
}

pub(crate) fn create_xr_session(
    xr_instance: &xr::Instance,
    system: xr::SystemId,
    vulkan_context: &VulkanContext,
) -> Result<(Session<Vulkan>, FrameWaiter, FrameStream<Vulkan>)> {
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

pub(crate) fn create_xr_instance() -> anyhow::Result<(xr::Instance, xr::SystemId)> {
    let xr_entry = xr::Entry::load()?;
    let xr_app_info = openxr::ApplicationInfo {
        application_name: "Hotham Asteroid",
        application_version: 1,
        engine_name: "Hotham",
        engine_version: 1,
    };
    let mut required_extensions = xr::ExtensionSet::default();
    // required_extensions.khr_vulkan_enable2 = true; // TODO: Should we use enable 2 for the simulator..?
    required_extensions.khr_vulkan_enable = true; // TODO: Should we use enable 2 for the simulator..?

    #[cfg(target_os = "android")]
    {
        required_extensions.khr_android_create_instance = true;
        xr_entry.initialize_android_loader()?;
    }

    println!(
        "Available extensions: {:?}",
        xr_entry.enumerate_extensions()?
    );

    let instance = xr_entry.create_instance(&xr_app_info, &required_extensions, &[])?;
    let system = instance.system(xr::FormFactor::HEAD_MOUNTED_DISPLAY)?;
    Ok((instance, system))
}

pub(crate) fn create_xr_instance_from_path(
    path: &std::path::Path,
) -> anyhow::Result<(xr::Instance, xr::SystemId)> {
    let xr_entry = xr::Entry::load_from(path)?;
    let xr_app_info = openxr::ApplicationInfo {
        application_name: "Hotham Asteroid",
        application_version: 1,
        engine_name: "Hotham",
        engine_version: 1,
    };
    let mut required_extensions = xr::ExtensionSet::default();
    // required_extensions.khr_vulkan_enable2 = true; // TODO: Should we use enable 2 for the simulator..?
    required_extensions.khr_vulkan_enable = true; // TODO: Should we use enable 2 for the simulator..?

    #[cfg(target_os = "android")]
    {
        required_extensions.khr_android_create_instance = true;
        xr_entry.initialize_android_loader()?;
    }

    println!(
        "Available extensions: {:?}",
        xr_entry.enumerate_extensions()?
    );

    let instance = xr_entry.create_instance(&xr_app_info, &required_extensions, &[])?;
    let system = instance.system(xr::FormFactor::HEAD_MOUNTED_DISPLAY)?;
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
