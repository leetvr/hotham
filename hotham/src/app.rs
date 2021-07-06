use crate::{
    renderer::Renderer, vulkan_context::VulkanContext, HothamResult, Program, BLEND_MODE,
    COLOR_FORMAT, VIEW_COUNT, VIEW_TYPE,
};
use anyhow::Result;
use ash::{
    version::InstanceV1_0,
    vk::{self, Handle},
};
use openxr as xr;

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::sleep,
    time::Duration,
};

#[cfg(target_os = "android")]
use std::{ffi::CStr, mem::transmute, ptr::null};

use xr::{
    vulkan_legacy::SessionCreateInfo, EventDataBuffer, FrameStream, FrameWaiter, Posef,
    ReferenceSpaceType, Session, SessionState, Swapchain, SwapchainCreateFlags,
    SwapchainCreateInfo, SwapchainUsageFlags, VulkanLegacy,
};

#[cfg(target_os = "android")]
use xr::sys::pfn::InitializeLoaderKHR;

#[cfg(target_os = "android")]
pub const ANDROID_LOOPER_ID_MAIN: u32 = 0;
#[cfg(target_os = "android")]
pub const ANDROID_LOOPER_ID_INPUT: u32 = 1;
#[cfg(target_os = "android")]
pub const ANDROID_LOOPER_NONBLOCKING_TIMEOUT: Duration = Duration::from_millis(0);
#[cfg(target_os = "android")]
pub const ANDROID_LOOPER_BLOCKING_TIMEOUT: Duration = Duration::from_millis(i32::MAX as _);

pub struct App<P: Program> {
    program: P,
    should_quit: Arc<AtomicBool>,
    renderer: Renderer,
    xr_instance: openxr::Instance,
    xr_session: Session<VulkanLegacy>,
    xr_state: SessionState,
    xr_swapchain: Swapchain<VulkanLegacy>,
    xr_space: xr::Space,
    _xr_action_set: xr::ActionSet,
    _xr_left_action: xr::Action<Posef>,
    _xr_right_action: xr::Action<Posef>,
    swapchain_resolution: vk::Extent2D,
    event_buffer: EventDataBuffer,
    frame_waiter: FrameWaiter,
    frame_stream: FrameStream<VulkanLegacy>,
    #[allow(dead_code)]
    resumed: bool,
}

impl<P> App<P>
where
    P: Program,
{
    pub fn new(mut program: P) -> HothamResult<Self> {
        let params = program.init()?;
        println!("[HOTHAM_APP] Initialised program!");

        let (xr_instance, system) = create_xr_instance()?;
        let vulkan_context = create_vulkan_context(&xr_instance, system)?;
        let (xr_session, frame_waiter, frame_stream) =
            create_xr_session(&xr_instance, system, &vulkan_context)?; // TODO: Extract to XRContext
        let xr_space =
            xr_session.create_reference_space(ReferenceSpaceType::STAGE, xr::Posef::IDENTITY)?;
        let swapchain_resolution = get_swapchain_resolution(&xr_instance, system)?;
        let xr_swapchain = create_xr_swapchain(&xr_session, &swapchain_resolution, VIEW_COUNT)?;
        // let _starfield_xr_swapchain = create_xr_swapchain(&xr_session, &swapchain_resolution, 1)?;

        // Create an action set to encapsulate our actions
        let xr_action_set = xr_instance.create_action_set("input", "input pose information", 0)?;

        let xr_right_action =
            xr_action_set.create_action::<xr::Posef>("right_hand", "Right Hand Controller", &[])?;
        let xr_left_action =
            xr_action_set.create_action::<xr::Posef>("left_hand", "Left Hand Controller", &[])?;

        // Bind our actions to input devices using the given profile
        // If you want to access inputs specific to a particular device you may specify a different
        // interaction profile
        xr_instance.suggest_interaction_profile_bindings(
            xr_instance
                .string_to_path("/interaction_profiles/oculus/touch_controller")
                .unwrap(),
            &[
                xr::Binding::new(
                    &xr_right_action,
                    xr_instance
                        .string_to_path("/user/hand/right/input/grip/pose")
                        .unwrap(),
                ),
                xr::Binding::new(
                    &xr_left_action,
                    xr_instance
                        .string_to_path("/user/hand/left/input/grip/pose")
                        .unwrap(),
                ),
            ],
        )?;

        // Attach the action set to the session
        xr_session.attach_action_sets(&[&xr_action_set])?;

        let renderer = Renderer::new(vulkan_context, &xr_swapchain, swapchain_resolution, &params)?;
        // renderer.

        Ok(Self {
            program,
            renderer,
            should_quit: Arc::new(AtomicBool::from(false)),
            xr_instance,
            xr_session,
            xr_swapchain,
            xr_space,
            xr_state: SessionState::IDLE,
            _xr_action_set: xr_action_set,
            _xr_left_action: xr_left_action,
            _xr_right_action: xr_right_action,
            swapchain_resolution,
            event_buffer: Default::default(),
            frame_stream,
            frame_waiter,
            resumed: true,
        })
    }

    pub fn run(&mut self) -> HothamResult<()> {
        #[cfg(not(target_os = "android"))]
        {
            let should_quit = self.should_quit.clone();
            ctrlc::set_handler(move || should_quit.store(true, Ordering::Relaxed))
                .map_err(anyhow::Error::new)?;
        }

        while !self.should_quit.load(Ordering::Relaxed) {
            #[cfg(target_os = "android")]
            self.process_android_events();

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

            self.xr_session.sync_actions(&[])?;

            // Wait for a frame to become available from the runtime
            let (frame_state, swapchain_image_index) = self.begin_frame()?;

            let (_, views) = self.xr_session.locate_views(
                VIEW_TYPE,
                frame_state.predicted_display_time,
                &self.xr_space,
            )?;

            if frame_state.should_render {
                self.renderer.update_uniform_buffer(&views, 10.0)?;
                self.renderer.draw(swapchain_image_index)?;
            }

            // Release the image back to OpenXR
            self.end_frame(frame_state, &views)?;
        }

        Ok(())
    }

    fn begin_frame(&mut self) -> Result<(xr::FrameState, usize)> {
        let frame_state = self.frame_waiter.wait()?;
        self.frame_stream.begin()?;

        let image_index = self.xr_swapchain.acquire_image()?;
        self.xr_swapchain.wait_image(openxr::Duration::INFINITE)?;

        Ok((frame_state, image_index as _))
    }

    fn end_frame(
        &mut self,
        frame_state: xr::FrameState,
        views: &Vec<xr::View>,
    ) -> openxr::Result<()> {
        self.xr_swapchain.release_image()?;

        let rect = xr::Rect2Di {
            offset: xr::Offset2Di { x: 0, y: 0 },
            extent: xr::Extent2Di {
                width: self.swapchain_resolution.width as _,
                height: self.swapchain_resolution.height as _,
            },
        };

        self.frame_stream.end(
            frame_state.predicted_display_time,
            BLEND_MODE,
            &[&xr::CompositionLayerProjection::new()
                .space(&self.xr_space)
                .views(&[
                    xr::CompositionLayerProjectionView::new()
                        .pose(views[0].pose)
                        .fov(views[0].fov)
                        .sub_image(
                            xr::SwapchainSubImage::new()
                                .swapchain(&self.xr_swapchain)
                                .image_array_index(0)
                                .image_rect(rect),
                        ),
                    xr::CompositionLayerProjectionView::new()
                        .pose(views[1].pose)
                        .fov(views[1].fov)
                        .sub_image(
                            xr::SwapchainSubImage::new()
                                .swapchain(&self.xr_swapchain)
                                .image_array_index(1)
                                .image_rect(rect),
                        ),
                ])],
        )
    }

    fn poll_xr_event(&mut self) -> Result<SessionState> {
        loop {
            match self.xr_instance.poll_event(&mut self.event_buffer)? {
                Some(xr::Event::SessionStateChanged(session_changed)) => {
                    let new_state = session_changed.state();

                    if self.xr_state == SessionState::IDLE && new_state == SessionState::READY {
                        println!("[HOTHAM_POLL_EVENT] Beginning session!");
                        self.xr_session.begin(VIEW_TYPE)?;
                    }

                    if self.xr_state != SessionState::STOPPING
                        && new_state == SessionState::STOPPING
                    {
                        println!("[HOTHAM_POLL_EVENT] Ending session!");
                        self.xr_session.end()?;
                    }

                    println!("[HOTHAM_POLL_EVENT] State is now {:?}", new_state);
                    self.xr_state = new_state;
                }
                Some(_) => {
                    println!("[HOTHAM_POLL_EVENT] Received some other event");
                }
                None => break,
            }
        }

        Ok(self.xr_state)
    }

    #[cfg(target_os = "android")]
    pub fn process_android_events(&mut self) {
        loop {
            if let Some(event) = self.poll_android_events() {
                println!("[HOTHAM_ANDROID] Received event {:?}", event);
                match event {
                    ndk_glue::Event::Resume => self.resumed = true,
                    ndk_glue::Event::Destroy => self.should_quit.store(true, Ordering::Relaxed),
                    ndk_glue::Event::Pause => self.resumed = false,
                    _ => {}
                }
            }
            break;
        }
    }

    #[cfg(target_os = "android")]
    pub fn poll_android_events(&mut self) -> Option<ndk_glue::Event> {
        use ndk::looper::{Poll, ThreadLooper};

        let looper = ThreadLooper::for_thread().unwrap();
        let timeout = if self.resumed {
            ANDROID_LOOPER_NONBLOCKING_TIMEOUT
        } else {
            ANDROID_LOOPER_BLOCKING_TIMEOUT
        };
        let result = looper.poll_all_timeout(timeout);

        match result {
            Ok(Poll::Event { ident, .. }) => {
                let ident = ident as u32;
                if ident == ANDROID_LOOPER_ID_MAIN {
                    ndk_glue::poll_events()
                } else if ident == ANDROID_LOOPER_ID_INPUT {
                    if let Some(input_queue) = ndk_glue::input_queue().as_ref() {
                        while let Some(event) = input_queue.get_event() {
                            if let Some(event) = input_queue.pre_dispatch(event) {
                                input_queue.finish_event(event, false);
                            }
                        }
                    }
                    None
                } else {
                    unreachable!(
                        "Unrecognised looper identifier: {:?} but LOOPER_ID_INPUT is {:?}",
                        ident, ANDROID_LOOPER_ID_INPUT
                    );
                }
            }
            _ => None,
        }
    }
}

#[cfg(target_os = "windows")]
fn create_vulkan_context(
    xr_instance: &xr::Instance,
    system: xr::SystemId,
) -> Result<VulkanContext, crate::hotham_error::HothamError> {
    let vulkan_context = VulkanContext::create_from_xr_instance(xr_instance, system)?;
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

fn get_swapchain_resolution(
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

fn create_xr_swapchain(
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

fn create_xr_session(
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
fn create_xr_instance() -> anyhow::Result<(xr::Instance, xr::SystemId)> {
    let xr_entry = xr::Entry::load()?;
    let xr_app_info = openxr::ApplicationInfo {
        application_name: "Hotham Cubeworld",
        application_version: 1,
        engine_name: "Hotham",
        engine_version: 1,
    };
    let mut required_extensions = xr::ExtensionSet::default();
    required_extensions.khr_vulkan_enable2 = true; // TODO: Should we use enable 2 for the simulator..?
    let instance = xr_entry.create_instance(&xr_app_info, &required_extensions, &[])?;
    let system = instance.system(xr::FormFactor::HEAD_MOUNTED_DISPLAY)?;
    Ok((instance, system))
}

#[cfg(target_os = "android")]
fn create_xr_instance() -> anyhow::Result<(xr::Instance, xr::SystemId)> {
    use anyhow::anyhow;
    use openxr::sys::{InstanceCreateInfoAndroidKHR, LoaderInitInfoAndroidKHR};

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
        application_name: "Hotham Cubeworld",
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
