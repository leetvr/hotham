use anyhow::Result;
use ash::vk::{self, Handle};
use openxr::{
    self as xr, EventDataBuffer, FrameStream, FrameWaiter, Session, SessionState, Space, Swapchain,
    Vulkan,
};
use xr::{
    vulkan::SessionCreateInfo, Duration, FrameState, ReferenceSpaceType, SwapchainCreateFlags,
    SwapchainUsageFlags, Time, View, ViewStateFlags,
};

use crate::{
    contexts::VulkanContext, util::is_view_valid, HothamError, HothamResult, BLEND_MODE,
    COLOR_FORMAT, VIEW_COUNT, VIEW_TYPE,
};

mod input;
use input::Input;

#[derive(Default)]
pub struct XrContextBuilder<'a> {
    path: Option<&'a std::path::Path>,
    application_name: Option<&'a str>,
    application_version: Option<u32>,
    required_extensions: Option<xr::ExtensionSet>,
}

impl<'a> XrContextBuilder<'a> {
    pub fn new() -> Self {
        XrContextBuilder::default()
    }

    pub fn path(&mut self, path: Option<&'a std::path::Path>) -> &mut Self {
        self.path = path;
        self
    }

    pub fn application_name(&mut self, name: Option<&'a str>) -> &mut Self {
        self.application_name = name;
        self
    }

    pub fn application_version(&mut self, version: Option<u32>) -> &mut Self {
        self.application_version = version;
        self
    }

    pub fn required_extensions(&mut self, extensions: Option<xr::ExtensionSet>) -> &mut Self {
        self.required_extensions = extensions;
        self
    }

    pub fn build(&mut self) -> Result<(XrContext, VulkanContext)> {
        let application_name = self.application_name.unwrap_or("Hotham Application");
        let application_version = self.application_version.unwrap_or(1);
        let (instance, system) = create_xr_instance(
            self.path,
            application_name,
            application_version,
            self.required_extensions.as_ref(),
        )?;
        XrContext::_new(instance, system, application_name, application_version)
    }
}

pub struct XrContext {
    pub instance: openxr::Instance,
    pub session: Session<Vulkan>,
    pub session_state: SessionState,
    pub swapchain: Swapchain<Vulkan>,
    pub stage_space: Space,
    pub view_space: Space,
    pub input: Input,
    pub swapchain_resolution: vk::Extent2D,
    pub frame_waiter: FrameWaiter,
    pub frame_stream: FrameStream<Vulkan>,
    pub frame_state: FrameState,
    pub views: Vec<View>,
    pub view_state_flags: ViewStateFlags,
}

impl XrContext {
    pub fn new() -> Result<(XrContext, VulkanContext)> {
        XrContextBuilder::new().build()
    }

    pub fn new_from_path<P: AsRef<std::path::Path>>(path: P) -> Result<(XrContext, VulkanContext)> {
        XrContextBuilder::new().path(Some(path.as_ref())).build()
    }

    #[cfg(test)]
    pub fn testing() -> (XrContext, VulkanContext) {
        XrContext::new_from_path("../openxr_loader.dll").unwrap()
    }

    fn _new(
        instance: xr::Instance,
        system: xr::SystemId,
        application_name: &str,
        application_version: u32,
    ) -> Result<(XrContext, VulkanContext)> {
        let vulkan_context =
            create_vulkan_context(&instance, system, application_name, application_version)?;

        let (session, frame_waiter, frame_stream) =
            create_xr_session(&instance, system, &vulkan_context)?;
        let stage_space =
            session.create_reference_space(ReferenceSpaceType::STAGE, xr::Posef::IDENTITY)?;
        let view_space =
            session.create_reference_space(ReferenceSpaceType::VIEW, xr::Posef::IDENTITY)?;
        let swapchain_resolution = get_swapchain_resolution(&instance, system)?;
        let swapchain = create_xr_swapchain(&session, &swapchain_resolution, VIEW_COUNT)?;

        let input = Input::oculus_touch_controller(&instance, &session)?;

        let frame_state = FrameState {
            predicted_display_time: Time::from_nanos(0),
            predicted_display_period: Duration::from_nanos(0),
            should_render: false,
        };

        // Attach the action set to the session
        session.attach_action_sets(&[&input.action_set])?;

        let xr_context = XrContext {
            instance,
            session,
            session_state: SessionState::IDLE,
            swapchain,
            stage_space,
            view_space,
            input,
            swapchain_resolution,
            frame_waiter,
            frame_stream,
            frame_state,
            views: vec![Default::default(); VIEW_COUNT as usize],
            view_state_flags: ViewStateFlags::EMPTY,
        };

        Ok((xr_context, vulkan_context))
    }

    pub(crate) fn poll_xr_event(
        &mut self,
        event_buffer: &mut EventDataBuffer,
    ) -> Result<SessionState> {
        match self.instance.poll_event(event_buffer)? {
            Some(xr::Event::SessionStateChanged(session_changed)) => {
                let new_state = session_changed.state();
                println!("[HOTHAM_POLL_EVENT] State is now {new_state:?}");
                self.session_state = new_state;
            }
            Some(xr::Event::InstanceLossPending(_)) => {
                println!("[HOTHAM_POLL_EVENT] Instance loss pending!");
            }
            Some(_) => println!("[HOTHAM_POLL_EVENT] Received some other event"),
            None => {}
        }

        Ok(self.session_state)
    }

    pub(crate) fn begin_frame(&mut self) -> HothamResult<usize> {
        self.frame_state = self.frame_waiter.wait()?;
        self.frame_stream.begin()?;

        if !self.frame_state.should_render {
            return Err(HothamError::NotRendering);
        }

        let image_index = self.swapchain.acquire_image()? as _;
        self.swapchain.wait_image(openxr::Duration::INFINITE)?;

        let active_action_set = xr::ActiveActionSet::new(&self.input.action_set);
        self.session.sync_actions(&[active_action_set])?;

        Ok(image_index)
    }

    pub fn update_views(&'_ mut self) -> &[xr::View] {
        let (view_state_flags, views) = self
            .session
            .locate_views(
                VIEW_TYPE,
                self.frame_state.predicted_display_time,
                &self.stage_space,
            )
            .unwrap();

        if is_view_valid(&view_state_flags) {
            self.views = views;
            self.view_state_flags = view_state_flags;
        }

        &self.views
    }

    pub fn end_frame(&mut self) -> std::result::Result<(), openxr::sys::Result> {
        // If we aren't in the rendering state, just submit empty views.
        if !self.frame_state.should_render {
            self.frame_stream
                .end(self.frame_state.predicted_display_time, BLEND_MODE, &[])
                .unwrap();
            return Ok(());
        }

        // Release the swapchain image.
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
            .space(&self.stage_space)
            .views(&views);

        let layers = [&*layer_projection];
        self.frame_stream.end(display_time, BLEND_MODE, &layers)
    }

    pub(crate) fn end_session(&mut self) -> anyhow::Result<()> {
        println!("[HOTHAM_XR] - Ending session..");
        self.session.end()?;
        println!("[HOTHAM_XR] - ..done!");
        Ok(())
    }
}

#[cfg(any(target_os = "android", feature = "editor"))]
pub(crate) fn create_vulkan_context(
    xr_instance: &xr::Instance,
    system: xr::SystemId,
    application_name: &str,
    application_version: u32,
) -> Result<VulkanContext, crate::hotham_error::HothamError> {
    let vulkan_context = VulkanContext::create_from_xr_instance(
        xr_instance,
        system,
        application_name,
        application_version,
    )?;
    println!("[HOTHAM_VULKAN] - Vulkan Context created successfully");
    Ok(vulkan_context)
}

#[cfg(all(not(target_os = "android"), not(feature = "editor")))]
fn create_vulkan_context(
    xr_instance: &xr::Instance,
    system: xr::SystemId,
    application_name: &str,
    application_version: u32,
) -> Result<VulkanContext, crate::hotham_error::HothamError> {
    #[allow(deprecated)]
    let vulkan_context = VulkanContext::create_from_xr_instance_legacy(
        xr_instance,
        system,
        application_name,
        application_version,
    )?;
    println!("[HOTHAM_VULKAN] - Vulkan Context created successfully");
    Ok(vulkan_context)
}

pub(crate) fn get_swapchain_resolution(
    xr_instance: &xr::Instance,
    system: xr::SystemId,
) -> Result<vk::Extent2D> {
    let views = xr_instance.enumerate_view_configuration_views(system, VIEW_TYPE)?;
    println!("[HOTHAM_VULKAN] Views: {views:?}");
    let resolution = vk::Extent2D {
        width: views[0].recommended_image_rect_width,
        height: views[0].recommended_image_rect_height,
    };

    Ok(resolution)
}

#[cfg(not(target_os = "android"))]
pub(crate) fn create_xr_swapchain(
    xr_session: &Session<Vulkan>,
    resolution: &vk::Extent2D,
    array_size: u32,
) -> Result<Swapchain<Vulkan>> {
    xr_session
        .create_swapchain(&xr::SwapchainCreateInfo {
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

/// Creates the OpenXR swapchain with Fixed Foveated Rendering support on Quest 2
///
/// This requires a fair bit of setup as there isn't yet a wrapper for this functionality in OpenXR.
#[cfg(target_os = "android")]
pub(crate) fn create_xr_swapchain(
    xr_session: &Session<Vulkan>,
    resolution: &vk::Extent2D,
    array_size: u32,
) -> Result<Swapchain<Vulkan>> {
    let mut swapchain_raw = xr::sys::Swapchain::NULL;
    let foveation_info = xr::sys::SwapchainCreateInfoFoveationFB {
        ty: xr::sys::StructureType::SWAPCHAIN_CREATE_INFO_FOVEATION_FB,
        next: std::ptr::null_mut(),
        flags: xr::sys::SwapchainCreateFoveationFlagsFB::FRAGMENT_DENSITY_MAP,
    };

    let create_info = xr::sys::SwapchainCreateInfo {
        ty: xr::sys::SwapchainCreateInfo::TYPE,
        create_flags: SwapchainCreateFlags::EMPTY,
        usage_flags: SwapchainUsageFlags::COLOR_ATTACHMENT,
        format: COLOR_FORMAT.as_raw() as _,
        sample_count: 1,
        width: resolution.width,
        height: resolution.height,
        face_count: 1,
        mip_count: 1,
        array_size,
        next: &foveation_info as *const _ as *const std::ffi::c_void,
    };

    unsafe {
        let fp = xr_session.instance().fp();
        let xr_result =
            (fp.create_swapchain)(xr_session.as_raw(), &create_info, &mut swapchain_raw);

        let swapchain = if xr_result.into_raw() >= 0 {
            Swapchain::from_raw(xr_session.clone(), swapchain_raw)
        } else {
            return Err(anyhow::Error::new(xr_result));
        };

        let fp = xr_session
            .instance()
            .exts()
            .fb_swapchain_update_state
            .unwrap();

        let foveation_profile = xr::FoveationLevelProfile {
            level: xr::FoveationLevelFB::HIGH,
            vertical_offset: 0.,
            dynamic: xr::FoveationDynamicFB::DISABLED,
        };

        let foveation_profile_handle =
            xr_session.create_foveation_profile(Some(foveation_profile))?;

        let swapchain_update_state = xr::sys::SwapchainStateFoveationFB {
            ty: xr::sys::SwapchainStateFoveationFB::TYPE,
            next: std::ptr::null_mut(),
            flags: xr::SwapchainStateFoveationFlagsFB::EMPTY,
            profile: foveation_profile_handle.as_raw(),
        };

        let result =
            (fp.update_swapchain)(swapchain_raw, std::mem::transmute(&swapchain_update_state));

        if result.into_raw() < 0 {
            return Err(anyhow::Error::new(result));
        }

        Ok(swapchain)
    }
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

pub(crate) fn create_xr_instance(
    path: Option<&std::path::Path>,
    application_name: &str,
    application_version: u32,
    required_extensions: Option<&xr::ExtensionSet>,
) -> anyhow::Result<(xr::Instance, xr::SystemId)> {
    let xr_entry = if let Some(path) = path {
        unsafe { xr::Entry::load_from(path)? }
    } else {
        unsafe { xr::Entry::load()? }
    };
    let xr_app_info = openxr::ApplicationInfo {
        application_name,
        application_version,
        engine_name: "Hotham",
        engine_version: 1,
    };

    let mut required_extensions = required_extensions.cloned().unwrap_or_default();
    enable_xr_extensions(&mut required_extensions);

    #[cfg(target_os = "android")]
    {
        xr_entry.initialize_android_loader()?;
    }

    let instance = xr_entry.create_instance(&xr_app_info, &required_extensions, &[])?;
    let system = instance.system(xr::FormFactor::HEAD_MOUNTED_DISPLAY)?;
    Ok((instance, system))
}

#[cfg(target_os = "android")]
fn enable_xr_extensions(required_extensions: &mut xr::ExtensionSet) {
    required_extensions.khr_android_create_instance = true;
    required_extensions.khr_vulkan_enable2 = true;
    required_extensions.fb_foveation = true;
    required_extensions.fb_foveation_configuration = true;
    required_extensions.fb_foveation_vulkan = true;
    required_extensions.fb_swapchain_update_state = true;
}

#[cfg(not(target_os = "android"))]
fn enable_xr_extensions(required_extensions: &mut xr::ExtensionSet) {
    if cfg!(feature = "editor") {
        required_extensions.khr_vulkan_enable2 = true;
    } else {
        required_extensions.khr_vulkan_enable = true;
    }
}

#[cfg(target_os = "windows")]
#[cfg(test)]
mod tests {
    use super::XrContext;

    #[test]
    pub fn test_xr_context_smoke_test() {
        XrContext::testing();
    }
}
