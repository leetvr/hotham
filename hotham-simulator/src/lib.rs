#![allow(
    non_snake_case,
    dead_code,
    non_upper_case_globals,
    non_camel_case_types
)]
mod openxr_loader;
use ash::{
    extensions::khr,
    util::read_spv,
    version::{DeviceV1_0, InstanceV1_0},
    vk::{
        self, DeviceCreateInfo, Handle, InstanceCreateInfo as VulkanInstanceCreateInfo,
        SwapchainKHR,
    },
    Device, Entry as AshEntry, Instance as AshInstance,
};
use lazy_static::lazy_static;
use openxr_loader::{
    PFN_xrEnumerateInstanceExtensionProperties, PFN_xrGetInstanceProcAddr, PFN_xrVoidFunction,
    XrExtensionProperties, XrInstance_T, XrResult,
};
use openxr_sys::{
    pfn,
    platform::{VkDevice, VkInstance, VkPhysicalDevice, VkResult},
    Action, ActionCreateInfo, ActionSet, ActionSetCreateInfo, ActionSpaceCreateInfo,
    ActionStateGetInfo, ActionStatePose, ActionsSyncInfo, Duration, EnvironmentBlendMode,
    EventDataBuffer, EventDataSessionStateChanged, Fovf, FrameBeginInfo, FrameEndInfo, FrameState,
    FrameWaitInfo, GraphicsRequirementsVulkanKHR, Instance, InstanceCreateInfo, InstanceProperties,
    InteractionProfileSuggestedBinding, Path, Posef, Quaternionf, ReferenceSpaceCreateInfo, Result,
    Session, SessionActionSetsAttachInfo, SessionBeginInfo, SessionCreateInfo, SessionState, Space,
    SpaceLocation, SpaceLocationFlags, StructureType, Swapchain, SwapchainCreateInfo,
    SwapchainImageAcquireInfo, SwapchainImageBaseHeader, SwapchainImageReleaseInfo,
    SwapchainImageVulkanKHR, SwapchainImageWaitInfo, SystemGetInfo, SystemId, Time, Vector3f,
    Version, View, ViewConfigurationType, ViewConfigurationView, ViewLocateInfo, ViewState,
    ViewStateFlags, VulkanDeviceCreateInfoKHR, VulkanGraphicsDeviceGetInfoKHR,
    VulkanInstanceCreateInfoKHR, TRUE,
};
use std::{
    ffi::{CStr, CString},
    fmt::Debug,
    intrinsics::{copy_nonoverlapping, transmute},
    io::Cursor,
    mem::size_of,
    os::raw::c_char,
    ptr::{self, null_mut},
    slice,
    sync::{atomic::AtomicBool, mpsc::channel, Arc, Mutex, MutexGuard},
    thread::{self, JoinHandle},
};
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::{run_return::EventLoopExtRunReturn, windows::EventLoopExtWindows},
    window::WindowBuilder,
};

static SWAPCHAIN_COLOUR_FORMAT: vk::Format = vk::Format::B8G8R8A8_SRGB;

struct State {
    vulkan_entry: Option<AshEntry>,
    vulkan_instance: Option<AshInstance>,
    physical_device: vk::PhysicalDevice,
    device: Option<Device>,
    session_state: SessionState,
    swapchain_fence: vk::Fence,
    internal_swapchain: SwapchainKHR,
    internal_swapchain_images: Vec<vk::Image>,
    internal_swapchain_image_views: Vec<vk::ImageView>,
    frame_count: usize,
    image_index: u32,
    present_queue: vk::Queue,
    present_queue_family_index: u32,
    command_pool: vk::CommandPool,
    multiview_images: Vec<vk::Image>,
    multiview_image_views: Vec<vk::ImageView>,
    multiview_images_memory: Vec<vk::DeviceMemory>,
    has_event: bool,
    command_buffers: Vec<vk::CommandBuffer>,
    pipelines: Vec<vk::Pipeline>,
    render_pass: vk::RenderPass,
    pipeline_layout: vk::PipelineLayout,
    descriptor_sets: Vec<vk::DescriptorSet>,
    descriptor_set_layout: vk::DescriptorSetLayout,
    framebuffers: Vec<vk::Framebuffer>,
    render_complete_semaphores: Vec<vk::Semaphore>,
    close_window: Arc<AtomicBool>,
    sampler: vk::Sampler,
    descriptor_pool: vk::DescriptorPool,
    surface: vk::SurfaceKHR,
    window_thread_handle: Option<JoinHandle<()>>,
}

impl Default for State {
    fn default() -> Self {
        State {
            vulkan_entry: None,
            vulkan_instance: None,
            physical_device: vk::PhysicalDevice::null(),
            device: None,
            session_state: SessionState::IDLE,
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
    unsafe fn destroy(&mut self) {
        println!("[HOTHAM_SIMULATOR] Destroy called..");
        let device = self.device.take().unwrap();
        device.device_wait_idle().unwrap();
        let instance = self.vulkan_instance.take().unwrap();
        let entry = self.vulkan_entry.take().unwrap();
        device.queue_wait_idle(self.present_queue).unwrap();
        for image in self.multiview_images.drain(..) {
            device.destroy_image(image, None)
        }
        device.destroy_fence(self.swapchain_fence, None);
        for memory in self.multiview_images_memory.drain(..) {
            device.free_memory(memory, None)
        }

        device.destroy_command_pool(self.command_pool, None);
        for semaphore in self.render_complete_semaphores.drain(..) {
            device.destroy_semaphore(semaphore, None)
        }

        for image_view in self.internal_swapchain_image_views.drain(..) {
            device.destroy_image_view(image_view, None)
        }

        for image_view in self.multiview_image_views.drain(..) {
            device.destroy_image_view(image_view, None)
        }

        device.destroy_pipeline_layout(self.pipeline_layout, None);
        for pipeline in self.pipelines.drain(..) {
            device.destroy_pipeline(pipeline, None);
        }

        let swapchain_ext = khr::Swapchain::new(&instance, &device);
        swapchain_ext.destroy_swapchain(self.internal_swapchain, None);

        device.destroy_render_pass(self.render_pass, None);
        device.destroy_sampler(self.sampler, None);
        device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        device.destroy_descriptor_pool(self.descriptor_pool, None);
        for framebuffer in self.framebuffers.drain(..) {
            device.destroy_framebuffer(framebuffer, None)
        }

        let surface_ext = khr::Surface::new(&entry, &instance);
        surface_ext.destroy_surface(self.surface, None);

        device.destroy_device(None);
        instance.destroy_instance(None);

        self.close_window
            .store(true, std::sync::atomic::Ordering::Relaxed);
        self.window_thread_handle.take().unwrap().join().unwrap();
        println!("[HOTHAM_SIMULATOR] All things are now destroyed");
    }
}

lazy_static! {
    static ref STATE: Mutex<State> = Default::default();
}

#[derive(Debug, Clone, Default)]
struct HothamSession {
    test: usize,
}

#[no_mangle]
unsafe extern "C" fn enumerate_instance_extension_properties(
    _layerName: *const ::std::os::raw::c_char,
    propertyCapacityInput: u32,
    propertyCountOutput: *mut u32,
    properties: *mut XrExtensionProperties,
) -> XrResult {
    if propertyCapacityInput == 0 {
        *propertyCountOutput = 1;
        return Result::SUCCESS.into_raw();
    }

    let extension = "XR_KHR_vulkan_enable2";
    let name = str_to_fixed_bytes(extension);
    let extensions = std::ptr::slice_from_raw_parts_mut(properties, 1);
    (*extensions)[0] = openxr_loader::XrExtensionProperties {
        type_: StructureType::EXTENSION_PROPERTIES.into_raw(),
        next: ptr::null_mut(),
        extensionName: name,
        extensionVersion: 2,
    };
    Result::SUCCESS.into_raw()
}

#[no_mangle]
unsafe extern "system" fn create_instance(
    _create_info: *const InstanceCreateInfo,
    instance: *mut Instance,
) -> Result {
    *instance = Instance::from_raw(42);

    Result::SUCCESS
}

unsafe extern "system" fn create_vulkan_instance(
    _instance: Instance,
    create_info: *const VulkanInstanceCreateInfoKHR,
    vulkan_instance: *mut VkInstance,
    vulkan_result: *mut VkResult,
) -> Result {
    let vulkan_create_info: &VulkanInstanceCreateInfo =
        transmute((*create_info).vulkan_create_info);
    let get_instance_proc_adddr = (*create_info).pfn_get_instance_proc_addr.unwrap();
    let vk_create_instance = CStr::from_bytes_with_nul_unchecked(b"vkCreateInstance\0").as_ptr();
    let create_instance: vk::PFN_vkCreateInstance =
        transmute(get_instance_proc_adddr(ptr::null(), vk_create_instance));
    let mut instance = vk::Instance::null();

    let event_loop: EventLoop<()> = EventLoop::new_any_thread();
    let window = WindowBuilder::new()
        .with_visible(false)
        .build(&event_loop)
        .unwrap();

    let mut create_info = *vulkan_create_info;
    let mut enabled_extensions = ash_window::enumerate_required_extensions(&window).unwrap();
    let xr_extensions = slice::from_raw_parts(
        create_info.pp_enabled_extension_names,
        create_info.enabled_extension_count as usize,
    );
    for ext in &(*xr_extensions) {
        enabled_extensions.push(CStr::from_ptr(*ext));
    }
    create_info.pp_enabled_layer_names =
        [CStr::from_bytes_with_nul_unchecked(b"VK_LAYER_KHRONOS_validation\0").as_ptr()].as_ptr();
    create_info.enabled_layer_count = 1;

    let enabled_extensions = enabled_extensions
        .iter()
        .map(|e| e.as_ptr())
        .collect::<Vec<_>>();
    create_info.enabled_extension_count = enabled_extensions.len() as _;
    create_info.pp_enabled_extension_names = enabled_extensions.as_ptr();

    let entry = AshEntry::new().unwrap();
    let result = create_instance(&create_info, ptr::null(), &mut instance);
    *vulkan_result = result.as_raw();
    if result != vk::Result::SUCCESS {
        return Result::ERROR_VALIDATION_FAILURE;
    }
    let static_fn = vk::StaticFn {
        get_instance_proc_addr: transmute(get_instance_proc_adddr),
    };
    let ash_instance = AshInstance::load(&static_fn, instance);

    *vulkan_instance = transmute(instance);

    let mut state = STATE.lock().unwrap();

    state.vulkan_entry.replace(entry);
    state.vulkan_instance.replace(ash_instance);
    Result::SUCCESS
}

unsafe extern "system" fn create_vulkan_device(
    _instance: Instance,
    create_info: *const VulkanDeviceCreateInfoKHR,
    vulkan_device: *mut VkDevice,
    vulkan_result: *mut VkResult,
) -> Result {
    *vulkan_result = ash::vk::Result::SUCCESS.as_raw();

    let create_info: &mut DeviceCreateInfo = transmute((*create_info).vulkan_create_info);
    println!(
        "[HOTHAM_SIMULATOR] Create vulkan device called with: {:?}",
        create_info
    );
    let mut extensions = slice::from_raw_parts(
        create_info.pp_enabled_extension_names,
        create_info.enabled_extension_count as usize,
    )
    .to_vec();
    extensions.push(khr::Swapchain::name().as_ptr());
    create_info.pp_enabled_extension_names = extensions.as_ptr();
    create_info.enabled_extension_count = extensions.len() as u32;

    println!(
        "[HOTHAM_SIMULATOR] Creating vulkan device with {:?}",
        create_info
    );
    let mut state = STATE.lock().unwrap();
    let vulkan_instance = state.vulkan_instance.as_ref().unwrap();
    let physical_device = state.physical_device;
    let device = vulkan_instance.create_device(physical_device, create_info, None);
    match device {
        Err(e) => {
            *vulkan_result = e.as_raw();
            return Result::SUCCESS;
        }
        _ => *vulkan_result = vk::Result::SUCCESS.as_raw(),
    }

    let device = device.unwrap();
    state.device = Some(device.clone());
    let info = vk::FenceCreateInfo::default();
    state.swapchain_fence = device.create_fence(&info, None).unwrap();
    let queue_family_index =
        slice::from_raw_parts(create_info.p_queue_create_infos, 1)[0].queue_family_index;
    state.command_pool = device
        .create_command_pool(
            &vk::CommandPoolCreateInfo::builder()
                .queue_family_index(queue_family_index)
                .flags(
                    vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER
                        | vk::CommandPoolCreateFlags::TRANSIENT,
                ),
            None,
        )
        .expect("Unable to create command pool");
    state.present_queue = device.get_device_queue(queue_family_index, 0);
    state.present_queue_family_index = queue_family_index;
    state.render_complete_semaphores = create_semaphores(&device);

    println!(
        "[HOTHAM_SIMULATOR] Done! Device created: {:?}",
        device.handle()
    );

    *vulkan_device = transmute(device.handle());
    Result::SUCCESS
}

unsafe fn create_semaphores(device: &Device) -> Vec<vk::Semaphore> {
    let semaphore_info = vk::SemaphoreCreateInfo::builder();
    (0..3)
        .map(|_| {
            device
                .create_semaphore(&semaphore_info, None)
                .expect("Unable to create semaphore")
        })
        .collect::<Vec<_>>()
}

unsafe extern "system" fn create_vulkan_physical_device(
    _instance: Instance,
    _get_info: *const VulkanGraphicsDeviceGetInfoKHR,
    vulkan_physical_device: *mut VkPhysicalDevice,
) -> Result {
    println!("[HOTHAM_SIMULATOR] Create vulkan physical device called");

    let mut state = STATE.lock().unwrap();
    let instance = state.vulkan_instance.as_ref().unwrap();

    let physical_device = instance
        .enumerate_physical_devices()
        .unwrap()
        .pop()
        .unwrap();

    println!(
        "[HOTHAM_SIMULATOR] Created physical device: {:?}",
        physical_device
    );
    *vulkan_physical_device = transmute(physical_device);

    state.physical_device = physical_device;
    Result::SUCCESS
}

unsafe extern "system" fn get_vulkan_graphics_requirements(
    _instance: Instance,
    _system_id: SystemId,
    graphics_requirements: *mut GraphicsRequirementsVulkanKHR,
) -> Result {
    *graphics_requirements = GraphicsRequirementsVulkanKHR {
        ty: GraphicsRequirementsVulkanKHR::TYPE,
        next: ptr::null_mut(),
        min_api_version_supported: Version::new(1, 1, 0),
        max_api_version_supported: Version::new(1, 1, 0),
    };
    Result::SUCCESS
}

unsafe extern "system" fn get_instance_properties(
    _instance: Instance,
    instance_properties: *mut InstanceProperties,
) -> Result {
    let runtime_name = str_to_fixed_bytes("Hotham Simulator");
    *instance_properties = InstanceProperties {
        ty: StructureType::INSTANCE_PROPERTIES,
        next: ptr::null_mut(),
        runtime_version: Version::new(0, 0, 1),
        runtime_name,
    };
    Result::SUCCESS
}

unsafe extern "system" fn enumerate_environment_blend_modes(
    _instance: Instance,
    _system_id: SystemId,
    _view_configuration_type: ViewConfigurationType,
    environment_blend_mode_capacity_input: u32,
    environment_blend_mode_count_output: *mut u32,
    environment_blend_modes: *mut EnvironmentBlendMode,
) -> Result {
    if environment_blend_mode_capacity_input == 0 {
        *environment_blend_mode_count_output = 1;
        return Result::SUCCESS;
    }
    let blend_modes = std::ptr::slice_from_raw_parts_mut(environment_blend_modes, 1);
    (*blend_modes)[0] = EnvironmentBlendMode::OPAQUE;
    Result::SUCCESS
}

unsafe extern "system" fn get_system(
    _instance: Instance,
    get_info: *const SystemGetInfo,
    system_id: *mut SystemId,
) -> Result {
    let get_info = *get_info;
    println!(
        "[HOTHAM_SIMULATOR] Get info called with {:?}",
        get_info.form_factor
    );
    *system_id = SystemId::from_raw(42);
    Result::SUCCESS
}

unsafe extern "system" fn create_session(
    _instance: Instance,
    _create_info: *const SessionCreateInfo,
    session: *mut Session,
) -> Result {
    let mut s = Box::new(HothamSession::default());
    s.test = 42;
    let s = Box::into_raw(s) as *const _;
    *session = Session::from_raw(s as _);
    Result::SUCCESS
}

fn create_pipeline_layout(state: &MutexGuard<State>) -> vk::PipelineLayout {
    let layouts = &[state.descriptor_set_layout];
    let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::builder().set_layouts(layouts);

    unsafe {
        state
            .device
            .as_ref()
            .unwrap()
            .create_pipeline_layout(&pipeline_layout_create_info, None)
            .expect("Unable to create pipeline layout")
    }
}

unsafe fn create_render_pass(state: &MutexGuard<State>) -> vk::RenderPass {
    let device = state.device.as_ref().unwrap();

    let color_attachment = vk::AttachmentDescription::builder()
        .format(SWAPCHAIN_COLOUR_FORMAT)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
        .samples(vk::SampleCountFlags::TYPE_1)
        .build();

    let color_attachment_ref = vk::AttachmentReference::builder()
        .attachment(0)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .build();

    let color_attachment_refs = [color_attachment_ref];

    let attachments = [color_attachment];

    let subpass = vk::SubpassDescription::builder()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(&color_attachment_refs)
        .build();
    let subpasses = [subpass];

    let dependency = vk::SubpassDependency::builder()
        .src_subpass(vk::SUBPASS_EXTERNAL)
        .dst_subpass(0)
        .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .src_access_mask(vk::AccessFlags::empty())
        .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
        .build();
    let dependencies = [dependency];

    let create_info = vk::RenderPassCreateInfo::builder()
        .attachments(&attachments)
        .subpasses(&subpasses)
        .dependencies(&dependencies);

    device
        .create_render_pass(&create_info, None)
        .expect("Unable to create render pass")
}

fn create_pipelines(state: &MutexGuard<State>) -> Vec<vk::Pipeline> {
    vec![create_pipeline(state, 0), create_pipeline(state, 1)]
}

fn create_pipeline(state: &MutexGuard<State>, i: usize) -> vk::Pipeline {
    let device = state.device.as_ref().unwrap();
    let pipeline_layout = state.pipeline_layout;
    let render_pass = state.render_pass;
    let vert_shader_code = read_spv(&mut Cursor::new(
        &include_bytes!("./shaders/viewdisplay.vert.spv")[..],
    ))
    .unwrap();
    let frag_shader_code = read_spv(&mut Cursor::new(
        &include_bytes!("./shaders/viewdisplay.frag.spv")[..],
    ))
    .unwrap();

    let name = CString::new("main").unwrap();
    let vertex_shader_module = unsafe {
        device.create_shader_module(
            &vk::ShaderModuleCreateInfo::builder().code(&vert_shader_code),
            None,
        )
    }
    .unwrap();
    let frag_shader_module = unsafe {
        device.create_shader_module(
            &vk::ShaderModuleCreateInfo::builder().code(&frag_shader_code),
            None,
        )
    }
    .unwrap();

    let rasterizer_create_info = vk::PipelineRasterizationStateCreateInfo::builder()
        .depth_clamp_enable(false)
        .rasterizer_discard_enable(false)
        .polygon_mode(vk::PolygonMode::FILL)
        .line_width(1.0)
        .cull_mode(vk::CullModeFlags::NONE)
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .depth_bias_enable(false);

    let vertex_shader_stage_info = vk::PipelineShaderStageCreateInfo::builder()
        .stage(vk::ShaderStageFlags::VERTEX)
        .module(vertex_shader_module)
        .name(name.as_c_str())
        .build();

    let map_entries = vk::SpecializationMapEntry::builder()
        .size(size_of::<f32>())
        .build();

    let input_assembly_create_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .primitive_restart_enable(false);

    let viewport_state_create_info = vk::PipelineViewportStateCreateInfo::builder()
        .viewport_count(1)
        .scissor_count(1);

    let multisampling_create_info = vk::PipelineMultisampleStateCreateInfo::builder()
        .sample_shading_enable(false)
        .rasterization_samples(vk::SampleCountFlags::TYPE_1)
        .min_sample_shading(1.0);

    let color_blend_attachment = vk::PipelineColorBlendAttachmentState::builder()
        .color_write_mask(
            vk::ColorComponentFlags::R
                | vk::ColorComponentFlags::G
                | vk::ColorComponentFlags::B
                | vk::ColorComponentFlags::A,
        )
        .blend_enable(false)
        .build();

    let color_blend_attachments = [color_blend_attachment];

    let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
        .logic_op_enable(false)
        .attachments(&color_blend_attachments);

    let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder().build();
    let data = i as f32;
    let specialization_info = vk::SpecializationInfo::builder()
        .data(&(data.to_ne_bytes()))
        .map_entries(&[map_entries])
        .build();

    let frag_shader_stage_info = vk::PipelineShaderStageCreateInfo::builder()
        .stage(vk::ShaderStageFlags::FRAGMENT)
        .module(frag_shader_module)
        .name(name.as_c_str())
        .specialization_info(&specialization_info)
        .build();

    let shader_stages = [vertex_shader_stage_info, frag_shader_stage_info];
    let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];

    let dynamic_state_info = vk::PipelineDynamicStateCreateInfo::builder()
        .dynamic_states(&dynamic_states)
        .build();

    let info = vk::GraphicsPipelineCreateInfo::builder()
        .stages(&shader_stages)
        .vertex_input_state(&vertex_input_state)
        .input_assembly_state(&input_assembly_create_info)
        .viewport_state(&viewport_state_create_info)
        .rasterization_state(&rasterizer_create_info)
        .multisample_state(&multisampling_create_info)
        .color_blend_state(&color_blend_state)
        .dynamic_state(&dynamic_state_info)
        .layout(pipeline_layout)
        .render_pass(render_pass)
        .subpass(0)
        .build();

    let create_infos = [info];
    let device = state.device.as_ref().unwrap();
    let pipeline =
        unsafe { device.create_graphics_pipelines(vk::PipelineCache::null(), &create_infos, None) }
            .expect("Unable to create pipeline")
            .pop()
            .unwrap();

    unsafe { device.destroy_shader_module(vertex_shader_module, None) };
    unsafe { device.destroy_shader_module(frag_shader_module, None) };
    pipeline
}

unsafe fn create_command_buffers(state: &MutexGuard<State>) -> Vec<vk::CommandBuffer> {
    let device = state.device.as_ref().unwrap();
    let command_pool = state.command_pool;
    let layout = state.pipeline_layout;
    let render_pass = state.render_pass;
    let pipelines = &state.pipelines;
    let framebuffers = &state.framebuffers;

    let allocate_info = vk::CommandBufferAllocateInfo::builder()
        .command_pool(command_pool)
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_buffer_count(3)
        .build();
    let command_buffers = device.allocate_command_buffers(&allocate_info).unwrap();
    let begin_info = vk::CommandBufferBeginInfo::builder().build();
    let extent = vk::Extent2D {
        width: 600,
        height: 600,
    };
    let render_area = vk::Rect2D {
        offset: vk::Offset2D { x: 0, y: 0 },
        extent,
    };
    let pipeline_bind_point = vk::PipelineBindPoint::GRAPHICS;

    for i in 0..command_buffers.len() {
        let descriptor_set = state.descriptor_sets[i];
        let command_buffer = &command_buffers[i];
        let framebuffer = &framebuffers[i];
        device
            .begin_command_buffer(*command_buffer, &begin_info)
            .expect("Unable to begin command buffer!");
        let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
            .clear_values(&[vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            }])
            .render_area(render_area)
            .framebuffer(*framebuffer)
            .render_pass(render_pass)
            .build();
        device.cmd_begin_render_pass(
            *command_buffer,
            &render_pass_begin_info,
            vk::SubpassContents::INLINE,
        );
        let mut viewport = vk::Viewport::builder()
            .height(extent.height as _)
            .width((extent.width / 2) as _)
            .max_depth(1 as _)
            .min_depth(0 as _)
            .build();
        let mut scissor = vk::Rect2D {
            extent,
            ..Default::default()
        };
        device.cmd_set_viewport(*command_buffer, 0, &[viewport]);
        device.cmd_set_scissor(*command_buffer, 0, &[scissor]);
        device.cmd_bind_descriptor_sets(
            *command_buffer,
            pipeline_bind_point,
            layout,
            0,
            &[descriptor_set],
            &[],
        );

        // Left Eye
        device.cmd_bind_pipeline(*command_buffer, pipeline_bind_point, pipelines[0]);
        device.cmd_draw(*command_buffer, 3, 1, 0, 0);

        // Right Eye
        viewport.x = 300 as _;
        scissor.offset.x = 300 as _;
        device.cmd_set_viewport(*command_buffer, 0, &[viewport]);
        device.cmd_set_scissor(*command_buffer, 0, &[scissor]);

        device.cmd_bind_pipeline(*command_buffer, pipeline_bind_point, pipelines[1]);
        device.cmd_draw(*command_buffer, 3, 1, 0, 0);

        device.cmd_end_render_pass(*command_buffer);
        device
            .end_command_buffer(*command_buffer)
            .expect("Unable to end command buffer");
    }
    command_buffers
}

unsafe extern "system" fn create_action_set(
    _instance: Instance,
    create_info: *const ActionSetCreateInfo,
    action_set: *mut ActionSet,
) -> Result {
    let create_info = *create_info;
    let name = CStr::from_ptr(create_info.action_set_name.as_ptr());
    println!(
        "[HOTHAM_SIMULATOR] Create action set called with {:?}",
        name
    );
    *action_set = ActionSet::from_raw(42);
    Result::SUCCESS
}

unsafe extern "system" fn create_action(
    action_set: ActionSet,
    _create_info: *const ActionCreateInfo,
    action: *mut Action,
) -> Result {
    println!(
        "[HOTHAM_SIMULATOR] Create action set called with {:?}",
        action_set
    );
    *action = Action::from_raw(42);
    Result::SUCCESS
}

unsafe extern "system" fn suggest_interaction_profile_bindings(
    _instance: Instance,
    _suggested_bindings: *const InteractionProfileSuggestedBinding,
) -> Result {
    println!("[HOTHAM_SIMULATOR] Suggest interaction profile bindings called!");
    Result::SUCCESS
}

unsafe extern "system" fn string_to_path(
    _instance: Instance,
    path_string: *const c_char,
    path: *mut Path,
) -> Result {
    let path_string = CStr::from_ptr(path_string);
    println!(
        "[HOTHAM_SIMULATOR] String to path called with {:?}",
        path_string
    );
    *path = Path::from_raw(42);
    Result::SUCCESS
}

unsafe extern "system" fn attach_action_sets(
    _session: Session,
    _attach_info: *const SessionActionSetsAttachInfo,
) -> Result {
    println!("[HOTHAM_SIMULATOR] Attach action sets called");
    Result::SUCCESS
}

unsafe extern "system" fn create_action_space(
    _session: Session,
    _create_info: *const ActionSpaceCreateInfo,
    space: *mut Space,
) -> Result {
    println!("[HOTHAM_SIMULATOR] Create action space called");
    *space = Space::from_raw(42);
    Result::SUCCESS
}

unsafe extern "system" fn create_reference_space(
    _session: Session,
    _create_info: *const ReferenceSpaceCreateInfo,
    space: *mut Space,
) -> Result {
    println!("[HOTHAM_SIMULATOR] Create reference space called");
    *space = Space::from_raw(42);
    Result::SUCCESS
}

unsafe extern "system" fn poll_event(
    _instance: Instance,
    event_data: *mut EventDataBuffer,
) -> Result {
    let mut state = STATE.lock().unwrap();
    if state.session_state == SessionState::IDLE {
        state.session_state = SessionState::READY;
        state.has_event = true;
    }

    if state.has_event {
        let data = EventDataSessionStateChanged {
            ty: StructureType::EVENT_DATA_SESSION_STATE_CHANGED,
            next: ptr::null(),
            session: Session::from_raw(42),
            state: state.session_state,
            time: openxr_sys::Time::from_nanos(10),
        };
        copy_nonoverlapping(&data, transmute(event_data), 1);
        state.has_event = false;

        Result::SUCCESS
    } else {
        Result::EVENT_UNAVAILABLE
    }
}

unsafe extern "system" fn begin_session(
    session: Session,
    _begin_info: *const SessionBeginInfo,
) -> Result {
    let ptr = session.into_raw() as *mut HothamSession;
    let s = Box::from_raw(ptr);
    println!("[HOTHAM_SIMULATOR] This is fucking stupid {:?}", s);
    Result::SUCCESS
}
unsafe extern "system" fn wait_frame(
    _session: Session,
    _frame_wait_info: *const FrameWaitInfo,
    frame_state: *mut FrameState,
) -> Result {
    let state = STATE.lock().unwrap();
    let _device = state.device.as_ref().unwrap();
    let _fence = state.swapchain_fence;

    // device.wait_for_fences(&[fence], true, u64::MAX).unwrap();
    *frame_state = FrameState {
        ty: StructureType::FRAME_STATE,
        next: ptr::null_mut(),
        predicted_display_time: Time::from_nanos(1),
        predicted_display_period: Duration::from_nanos(1),
        should_render: TRUE,
    };
    Result::SUCCESS
}

unsafe extern "system" fn begin_frame(
    _session: Session,
    _frame_begin_info: *const FrameBeginInfo,
) -> Result {
    Result::SUCCESS
}

unsafe extern "system" fn enumerate_view_configuration_views(
    _instance: Instance,
    _system_id: SystemId,
    _view_configuration_type: ViewConfigurationType,
    view_capacity_input: u32,
    view_count_output: *mut u32,
    views: *mut ViewConfigurationView,
) -> Result {
    if view_capacity_input == 0 {
        *view_count_output = 2;
        return Result::SUCCESS;
    }

    let views = std::ptr::slice_from_raw_parts_mut(views, 2);
    for i in 0..2 {
        (*views)[i] = ViewConfigurationView {
            ty: StructureType::VIEW_CONFIGURATION_VIEW,
            next: null_mut(),
            recommended_image_rect_width: 600,
            max_image_rect_width: 600,
            recommended_image_rect_height: 600,
            max_image_rect_height: 600,
            recommended_swapchain_sample_count: 3,
            max_swapchain_sample_count: 3,
        };
    }
    Result::SUCCESS
}

unsafe extern "system" fn create_xr_swapchain(
    _session: Session,
    create_info: *const SwapchainCreateInfo,
    swapchain: *mut Swapchain,
) -> Result {
    println!("[HOTHAM_SIMULATOR] Creating XR Swapchain..");
    let mut state = STATE.lock().unwrap();
    let format = vk::Format::from_raw((*create_info).format as _);
    let (multiview_images, multiview_images_memory) =
        create_multiview_images(&state, &(*create_info));
    println!("[HOTHAM_SIMULATOR] ..done.");

    state.multiview_images = multiview_images;
    state.multiview_images_memory = multiview_images_memory;
    state.multiview_image_views = create_multiview_image_views(&state, format);

    println!("[HOTHAM_SIMULATOR] Building windows swapchain..");
    let windows_swapchain = build_swapchain(&mut state);
    println!("[HOTHAM_SIMULATOR] ..done");
    let s = Swapchain::from_raw(windows_swapchain.as_raw());
    println!("[HOTHAM_SIMULATOR] Returning with {:?}", s);
    *swapchain = s;
    Result::SUCCESS
}

fn create_multiview_image_views(
    state: &MutexGuard<State>,
    format: vk::Format,
) -> Vec<vk::ImageView> {
    let device = state.device.as_ref().unwrap();
    let aspect_mask = vk::ImageAspectFlags::COLOR;
    state
        .multiview_images
        .iter()
        .map(|image| {
            let subresource_range = vk::ImageSubresourceRange::builder()
                .aspect_mask(aspect_mask)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(2)
                .build();

            let create_info = vk::ImageViewCreateInfo::builder()
                .image(*image)
                .view_type(vk::ImageViewType::TYPE_2D_ARRAY)
                .format(format)
                .subresource_range(subresource_range);

            unsafe {
                device
                    .create_image_view(&create_info, None)
                    .expect("Unable to create image view")
            }
        })
        .collect::<Vec<_>>()
}

unsafe fn build_swapchain(state: &mut MutexGuard<State>) -> SwapchainKHR {
    let entry = state.vulkan_entry.as_ref().unwrap().clone();
    let instance = state.vulkan_instance.as_ref().unwrap().clone();
    let device = state.device.as_ref().unwrap();
    let physical_device = state.physical_device;
    let swapchain_ext = khr::Swapchain::new(&instance, device);
    let queue_family_index = state.present_queue_family_index;
    let close_window = state.close_window.clone();

    let (tx, rx) = channel();
    let window_thread_handle = thread::spawn(move || {
        let mut event_loop: EventLoop<()> = EventLoop::new_any_thread();
        println!("[HOTHAM_SIMULATOR] Creating window..");
        let visible = false;
        let window = WindowBuilder::new()
            .with_inner_size(PhysicalSize::new(600, 600))
            .with_title("Hotham Simulator")
            .with_visible(visible)
            .build(&event_loop)
            .unwrap();
        println!("[HOTHAM_SIMULATOR] ..done.");
        let extent = vk::Extent2D {
            height: 600,
            width: 600,
        };

        println!("[HOTHAM_SIMULATOR] Creating surface..");
        let surface = ash_window::create_surface(&entry, &instance, &window, None).unwrap();
        println!("[HOTHAM_SIMULATOR] ..done");
        let swapchain_support_details = SwapChainSupportDetails::query_swap_chain_support(
            &entry,
            &instance,
            physical_device,
            surface,
            queue_family_index,
        );

        let create_info = vk::SwapchainCreateInfoKHR::builder()
            .min_image_count(3)
            .surface(surface)
            .image_format(SWAPCHAIN_COLOUR_FORMAT)
            .image_color_space(vk::ColorSpaceKHR::SRGB_NONLINEAR)
            .image_array_layers(1)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .image_extent(extent)
            .queue_family_indices(&[])
            .pre_transform(swapchain_support_details.capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(vk::PresentModeKHR::MAILBOX)
            .clipped(true)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT);

        println!("[HOTHAM_SIMULATOR] About to create swapchain..");
        let swapchain = swapchain_ext
            .clone()
            .create_swapchain(&create_info, None)
            .unwrap();
        println!(
            "[HOTHAM_SIMULATOR] Created swapchain: {:?}. Sending..",
            swapchain
        );
        tx.send((surface, swapchain)).unwrap();

        if !visible {
            return;
        }
        let cl2 = close_window.clone();

        event_loop.run_return(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            if close_window.load(std::sync::atomic::Ordering::Relaxed) {
                println!("[HOTHAM_SIMULATOR] Closed called!");
                *control_flow = ControlFlow::Exit;
            }

            match event {
                Event::WindowEvent { event, window_id } if window_id == window.id() => {
                    match event {
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                        _ => {}
                    }
                }
                Event::LoopDestroyed => {}
                Event::MainEventsCleared => {
                    window.request_redraw();
                }
                Event::RedrawRequested(_window_id) => {}
                _ => (),
            }
        });

        cl2.store(true, std::sync::atomic::Ordering::Relaxed);
    });
    let (surface, swapchain) = rx.recv().unwrap();
    println!("[HOTHAM_SIMULATOR] Received swapchain: {:?}", swapchain);
    let instance = state.vulkan_instance.as_ref().unwrap().clone();
    let swapchain_ext = khr::Swapchain::new(&instance, device);

    state.surface = surface;
    state.window_thread_handle = Some(window_thread_handle);
    state.internal_swapchain = swapchain;
    state.internal_swapchain_images = swapchain_ext
        .get_swapchain_images(swapchain)
        .expect("Unable to get swapchain images");
    state.internal_swapchain_image_views = create_swapchain_image_views(state);

    state.descriptor_sets = create_descriptor_sets(state);
    println!("[HOTHAM_SIMULATOR] Creating render pass..");
    state.render_pass = create_render_pass(state);
    println!("[HOTHAM_SIMULATOR] ..done!");
    state.framebuffers = create_framebuffers(state);
    state.pipeline_layout = create_pipeline_layout(state);
    println!("[HOTHAM_SIMULATOR] Creating pipelines..");
    state.pipelines = create_pipelines(state);
    println!("[HOTHAM_SIMULATOR] ..done!");
    state.command_buffers = create_command_buffers(state);
    swapchain
}

unsafe fn create_descriptor_sets(state: &mut MutexGuard<State>) -> Vec<vk::DescriptorSet> {
    let device = state.device.as_ref().unwrap();
    let image_views = &state.multiview_image_views;
    // descriptor pool
    let descriptor_pool = device
        .create_descriptor_pool(
            &vk::DescriptorPoolCreateInfo::builder()
                .pool_sizes(&[vk::DescriptorPoolSize::builder()
                    .descriptor_count(3)
                    .ty(vk::DescriptorType::SAMPLER)
                    .build()])
                .max_sets(3)
                .build(),
            None,
        )
        .expect("Unable to create desctiptor pool");

    let bindings = [vk::DescriptorSetLayoutBinding::builder()
        .binding(0)
        .descriptor_count(1)
        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .stage_flags(vk::ShaderStageFlags::FRAGMENT)
        .build()];

    // descriptor layout
    let layout = device
        .create_descriptor_set_layout(
            &vk::DescriptorSetLayoutCreateInfo::builder()
                .bindings(&bindings)
                .build(),
            None,
        )
        .expect("Unable to create descriptor set layouts");

    let set_layouts = [layout, layout, layout];

    // allocate
    let descriptor_sets = device
        .allocate_descriptor_sets(
            &vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(descriptor_pool)
                .set_layouts(&set_layouts)
                .build(),
        )
        .expect("Unable to create descriptor sets");

    let create_info = vk::SamplerCreateInfo::builder()
        .mag_filter(vk::Filter::LINEAR)
        .min_filter(vk::Filter::LINEAR)
        .address_mode_u(vk::SamplerAddressMode::REPEAT)
        .address_mode_v(vk::SamplerAddressMode::REPEAT)
        .address_mode_w(vk::SamplerAddressMode::REPEAT)
        .anisotropy_enable(false)
        .max_anisotropy(16.0)
        .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
        .unnormalized_coordinates(false)
        .compare_enable(false)
        .compare_op(vk::CompareOp::ALWAYS)
        .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
        .mip_lod_bias(0.0)
        .min_lod(0.0)
        .max_lod(0.0)
        .build();

    let sampler = device
        .create_sampler(&create_info, None)
        .expect("Unable to create sampler");

    for i in 0..descriptor_sets.len() {
        let descriptor_set = descriptor_sets[i];
        let image_info = vk::DescriptorImageInfo::builder()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(image_views[i])
            .sampler(sampler)
            .build();

        let sampler_descriptor_write = vk::WriteDescriptorSet::builder()
            .dst_set(descriptor_set)
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(&[image_info])
            .build();

        device.update_descriptor_sets(&[sampler_descriptor_write], &[])
    }

    // return

    state.descriptor_set_layout = set_layouts[0];
    state.sampler = sampler;
    state.descriptor_pool = descriptor_pool;

    descriptor_sets
}

fn create_swapchain_image_views(state: &mut MutexGuard<State>) -> Vec<vk::ImageView> {
    let device = state.device.as_ref().unwrap();
    let aspect_mask = vk::ImageAspectFlags::COLOR;
    state
        .internal_swapchain_images
        .iter()
        .map(|image| {
            let subresource_range = vk::ImageSubresourceRange::builder()
                .aspect_mask(aspect_mask)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1)
                .build();

            let create_info = vk::ImageViewCreateInfo::builder()
                .image(*image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(SWAPCHAIN_COLOUR_FORMAT)
                .subresource_range(subresource_range);

            unsafe {
                device
                    .create_image_view(&create_info, None)
                    .expect("Unable to create image view")
            }
        })
        .collect::<Vec<_>>()
}

fn create_framebuffers(state: &mut MutexGuard<State>) -> Vec<vk::Framebuffer> {
    let device = state.device.as_ref().unwrap();
    let render_pass = state.render_pass;
    state
        .internal_swapchain_image_views
        .iter()
        .map(|image_view| {
            let attachments = &[*image_view];
            let create_info = vk::FramebufferCreateInfo::builder()
                .render_pass(render_pass)
                .attachments(attachments)
                .width(600)
                .height(600)
                .layers(1);

            unsafe { device.create_framebuffer(&create_info, None).unwrap() }
        })
        .collect::<Vec<_>>()
}

unsafe extern "system" fn enumerate_swapchain_images(
    _swapchain: Swapchain,
    image_capacity_input: u32,
    image_count_output: *mut u32,
    images: *mut SwapchainImageBaseHeader,
) -> Result {
    if image_capacity_input == 0 {
        *image_count_output = 3;
        return Result::SUCCESS;
    }
    println!("[HOTHAM_SIMULATOR] Creating swapchain images..");
    let multiview_images = &STATE.lock().unwrap().multiview_images;

    let images = slice::from_raw_parts_mut(images as _, 3);
    for i in 0..3 {
        let image = multiview_images[i];
        images[i] = SwapchainImageVulkanKHR {
            ty: StructureType::SWAPCHAIN_IMAGE_VULKAN_KHR,
            next: null_mut(),
            image: image.as_raw(),
        };
    }

    println!("[HOTHAM_SIMULATOR] Done!");

    Result::SUCCESS
}

fn create_multiview_images(
    state: &MutexGuard<State>,
    create_info: &SwapchainCreateInfo,
) -> (Vec<vk::Image>, Vec<vk::DeviceMemory>) {
    let device = state.device.as_ref().unwrap();
    let instance = state.vulkan_instance.as_ref().unwrap();
    let physical_device = state.physical_device;

    let extent = vk::Extent3D {
        width: create_info.width,
        height: create_info.height,
        depth: 1,
    };
    let format = vk::Format::from_raw(create_info.format as _);
    let tiling = vk::ImageTiling::OPTIMAL;
    let usage = vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED;
    let properties = vk::MemoryPropertyFlags::DEVICE_LOCAL;

    let create_info = vk::ImageCreateInfo::builder()
        .image_type(vk::ImageType::TYPE_2D)
        .extent(extent)
        .mip_levels(1)
        .array_layers(2)
        .format(format)
        .tiling(tiling)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .usage(usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .samples(vk::SampleCountFlags::TYPE_1);

    let mut images = Vec::new();
    let mut device_memory = Vec::new();

    for _ in 0..3 {
        let image = unsafe {
            device
                .create_image(&create_info, None)
                .expect("Unable to create image")
        };
        let memory_requirements = unsafe { device.get_image_memory_requirements(image) };
        let memory_type_index = find_memory_type(
            instance,
            physical_device,
            memory_requirements.memory_type_bits,
            properties,
        );
        let alloc_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(memory_requirements.size)
            .memory_type_index(memory_type_index);

        let image_memory = unsafe {
            device
                .allocate_memory(&alloc_info, None)
                .expect("Unable to allocate memory")
        };
        unsafe {
            device
                .bind_image_memory(image, image_memory, 0)
                .expect("Unable to bind memory")
        };
        images.push(image);
        device_memory.push(image_memory);
    }

    (images, device_memory)
}

unsafe extern "system" fn acquire_swapchain_image(
    swapchain: Swapchain,
    _acquire_info: *const SwapchainImageAcquireInfo,
    index: *mut u32,
) -> Result {
    println!("[HOTHAM_SIMULATOR] Acquire swapchain image called..");
    let swapchain = vk::SwapchainKHR::from_raw(swapchain.into_raw());
    let state = STATE.lock().unwrap();
    let device = state.device.as_ref().unwrap();
    let ext = khr::Swapchain::new(state.vulkan_instance.as_ref().unwrap(), device);
    let fence = state.swapchain_fence;
    device
        .reset_fences(&[fence])
        .expect("Failed to reset fence");

    let (i, _) = ext
        .acquire_next_image(swapchain, u64::MAX - 1, vk::Semaphore::null(), fence)
        .unwrap();
    device
        .wait_for_fences(&[fence], true, u64::MAX)
        .expect("Failed to wait for fence");
    drop(state);

    *index = i;

    let mut state = STATE.lock().unwrap();
    state.image_index = i;
    println!("[HOTHAM_SIMULATOR] Done. Index is {}", i);
    Result::SUCCESS
}

unsafe extern "system" fn wait_swapchain_image(
    _swapchain: Swapchain,
    _wait_info: *const SwapchainImageWaitInfo,
) -> Result {
    Result::SUCCESS
}

unsafe extern "system" fn dummy() -> Result {
    println!("[HOTHAM_SIMULATOR] Uh oh, dummy called");
    Result::SUCCESS
}

unsafe extern "system" fn locate_space(
    _space: Space,
    _base_space: Space,
    _time: Time,
    location: *mut SpaceLocation,
) -> Result {
    *location = SpaceLocation {
        ty: StructureType::SPACE_LOCATION,
        next: null_mut(),
        location_flags: SpaceLocationFlags::ORIENTATION_TRACKED,
        pose: Posef {
            orientation: Quaternionf::IDENTITY,
            position: Vector3f::default(),
        },
    };
    Result::SUCCESS
}
unsafe extern "system" fn get_action_state_pose(
    _session: Session,
    _get_info: *const ActionStateGetInfo,
    state: *mut ActionStatePose,
) -> Result {
    *state = ActionStatePose {
        ty: StructureType::ACTION_STATE_POSE,
        next: null_mut(),
        is_active: TRUE,
    };
    Result::SUCCESS
}

unsafe extern "system" fn sync_actions(
    _session: Session,
    _sync_info: *const ActionsSyncInfo,
) -> Result {
    Result::SUCCESS
}

unsafe extern "system" fn locate_views(
    _session: Session,
    _view_locate_info: *const ViewLocateInfo,
    view_state: *mut ViewState,
    view_capacity_input: u32,
    view_count_output: *mut u32,
    views: *mut View,
) -> Result {
    if view_capacity_input == 0 {
        *view_count_output = 2;
        return Result::SUCCESS;
    }

    *view_state = ViewState {
        ty: StructureType::VIEW_STATE,
        next: null_mut(),
        view_state_flags: ViewStateFlags::ORIENTATION_VALID,
    };
    let views = slice::from_raw_parts_mut(views, 2);
    for i in 0..2 {
        views[i] = View {
            ty: StructureType::VIEW,
            next: null_mut(),
            pose: Posef {
                orientation: Quaternionf::IDENTITY,
                position: Vector3f::default(),
            },
            fov: Fovf::default(),
        };
    }

    Result::SUCCESS
}

unsafe extern "system" fn release_swapchain_image(
    _swapchain: Swapchain,
    _release_info: *const SwapchainImageReleaseInfo,
) -> Result {
    println!("[HOTHAM_SIMULATOR] Release image called");
    Result::SUCCESS
}

unsafe extern "system" fn end_frame(
    _session: Session,
    _frame_end_info: *const FrameEndInfo,
) -> Result {
    let mut state = STATE.lock().unwrap();
    let instance = state.vulkan_instance.as_ref().unwrap();
    let device = state.device.as_ref().unwrap();
    let swapchain = state.internal_swapchain;
    let swapchains = [swapchain];
    let queue = state.present_queue;
    let index = state.image_index as usize;
    let command_buffers = [state.command_buffers[index]];
    let image = state.multiview_images[index];
    transition_image_layout(
        device,
        queue,
        state.command_pool,
        image,
        vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
    );

    let image_indices = [index as u32];
    let render_complete = [state.render_complete_semaphores[index]];

    let submit_info = vk::SubmitInfo::builder()
        .command_buffers(&command_buffers)
        .signal_semaphores(&render_complete)
        .build();

    let submits = [submit_info];

    device
        .queue_submit(state.present_queue, &submits, vk::Fence::null())
        .expect("Unable to submit to queue");

    let present_info = vk::PresentInfoKHR::builder()
        .wait_semaphores(&render_complete)
        .swapchains(&swapchains)
        .image_indices(&image_indices);

    let ext = khr::Swapchain::new(instance, device);

    println!(
        "[HOTHAM_SIMULATOR] About to present frame {}",
        state.frame_count
    );
    ext.queue_present(queue, &present_info).unwrap();
    println!("[HOTHAM_SIMULATOR] done! Probably?");

    state.frame_count += 1;
    Result::SUCCESS
}

unsafe extern "system" fn request_exit_session(_session: Session) -> Result {
    let mut state = STATE.lock().unwrap();
    state.session_state = SessionState::EXITING;
    state.has_event = true;
    Result::SUCCESS
}

unsafe extern "system" fn destroy_space(_space: Space) -> Result {
    Result::SUCCESS
}

unsafe extern "system" fn destroy_action(_action: Action) -> Result {
    Result::SUCCESS
}

unsafe extern "system" fn destroy_action_set(_action_set: ActionSet) -> Result {
    Result::SUCCESS
}

unsafe extern "system" fn destroy_swapchain(_swapchain: Swapchain) -> Result {
    Result::SUCCESS
}

unsafe extern "system" fn destroy_session(_session: Session) -> Result {
    STATE.lock().unwrap().destroy();

    Result::SUCCESS
}

unsafe extern "system" fn destroy_instance(_instance: Instance) -> Result {
    Result::SUCCESS
}

type DummyFn = unsafe extern "system" fn() -> Result;

#[no_mangle]
pub unsafe extern "C" fn get_instance_proc_addr(
    _instance: *mut XrInstance_T,
    name: *const i8,
    function: *mut PFN_xrVoidFunction,
) -> XrResult {
    let name = CStr::from_ptr(name);
    let name = name.to_bytes();
    if name == b"xrGetInstanceProcAddr" {
        *function = transmute::<PFN_xrGetInstanceProcAddr, _>(Some(get_instance_proc_addr));
    } else if name == b"xrEnumerateInstanceExtensionProperties" {
        *function = transmute::<PFN_xrEnumerateInstanceExtensionProperties, _>(Some(
            enumerate_instance_extension_properties,
        ));
    } else if name == b"xrCreateInstance" {
        *function = transmute::<pfn::CreateInstance, _>(create_instance);
    } else if name == b"xrCreateVulkanInstanceKHR" {
        *function = transmute::<pfn::CreateVulkanInstanceKHR, _>(create_vulkan_instance);
    } else if name == b"xrCreateVulkanDeviceKHR" {
        *function = transmute::<pfn::CreateVulkanDeviceKHR, _>(create_vulkan_device);
    } else if name == b"xrGetVulkanGraphicsDevice2KHR" {
        *function = transmute::<pfn::GetVulkanGraphicsDevice2KHR, _>(create_vulkan_physical_device);
    } else if name == b"xrGetInstanceProperties" {
        *function = transmute::<pfn::GetInstanceProperties, _>(get_instance_properties);
    } else if name == b"xrGetVulkanGraphicsRequirements2KHR" {
        *function = transmute::<pfn::GetVulkanGraphicsRequirements2KHR, _>(
            get_vulkan_graphics_requirements,
        );
    } else if name == b"xrEnumerateEnvironmentBlendModes" {
        *function =
            transmute::<pfn::EnumerateEnvironmentBlendModes, _>(enumerate_environment_blend_modes);
    } else if name == b"xrGetSystem" {
        *function = transmute::<pfn::GetSystem, _>(get_system);
    } else if name == b"xrCreateSession" {
        *function = transmute::<pfn::CreateSession, _>(create_session);
    } else if name == b"xrCreateActionSet" {
        *function = transmute::<pfn::CreateActionSet, _>(create_action_set);
    } else if name == b"xrCreateAction" {
        *function = transmute::<pfn::CreateAction, _>(create_action);
    } else if name == b"xrSuggestInteractionProfileBindings" {
        *function = transmute::<pfn::SuggestInteractionProfileBindings, _>(
            suggest_interaction_profile_bindings,
        );
    } else if name == b"xrStringToPath" {
        *function = transmute::<pfn::StringToPath, _>(string_to_path);
    } else if name == b"xrAttachSessionActionSets" {
        *function = transmute::<pfn::AttachSessionActionSets, _>(attach_action_sets);
    } else if name == b"xrCreateActionSpace" {
        *function = transmute::<pfn::CreateActionSpace, _>(create_action_space);
    } else if name == b"xrCreateReferenceSpace" {
        *function = transmute::<pfn::CreateReferenceSpace, _>(create_reference_space);
    } else if name == b"xrPollEvent" {
        *function = transmute::<pfn::PollEvent, _>(poll_event);
    } else if name == b"xrBeginSession" {
        *function = transmute::<pfn::BeginSession, _>(begin_session);
    } else if name == b"xrWaitFrame" {
        *function = transmute::<pfn::WaitFrame, _>(wait_frame);
    } else if name == b"xrBeginFrame" {
        *function = transmute::<pfn::BeginFrame, _>(begin_frame);
    } else if name == b"xrEnumerateViewConfigurationViews" {
        *function = transmute::<pfn::EnumerateViewConfigurationViews, _>(
            enumerate_view_configuration_views,
        );
    } else if name == b"xrCreateSwapchain" {
        *function = transmute::<pfn::CreateSwapchain, _>(create_xr_swapchain);
    } else if name == b"xrEnumerateSwapchainImages" {
        *function = transmute::<pfn::EnumerateSwapchainImages, _>(enumerate_swapchain_images);
    } else if name == b"xrAcquireSwapchainImage" {
        *function = transmute::<pfn::AcquireSwapchainImage, _>(acquire_swapchain_image);
    } else if name == b"xrWaitSwapchainImage" {
        *function = transmute::<pfn::WaitSwapchainImage, _>(wait_swapchain_image);
    } else if name == b"xrSyncActions" {
        *function = transmute::<pfn::SyncActions, _>(sync_actions);
    } else if name == b"xrLocateSpace" {
        *function = transmute::<pfn::LocateSpace, _>(locate_space);
    } else if name == b"xrGetActionStatePose" {
        *function = transmute::<pfn::GetActionStatePose, _>(get_action_state_pose);
    } else if name == b"xrLocateViews" {
        *function = transmute::<pfn::LocateViews, _>(locate_views);
    } else if name == b"xrReleaseSwapchainImage" {
        *function = transmute::<pfn::ReleaseSwapchainImage, _>(release_swapchain_image);
    } else if name == b"xrEndFrame" {
        *function = transmute::<pfn::EndFrame, _>(end_frame);
    } else if name == b"xrRequestExitSession" {
        *function = transmute::<pfn::RequestExitSession, _>(request_exit_session);
    } else if name == b"xrDestroySpace" {
        *function = transmute::<pfn::DestroySpace, _>(destroy_space);
    } else if name == b"xrDestroyAction" {
        *function = transmute::<pfn::DestroyAction, _>(destroy_action);
    } else if name == b"xrDestroyActionSet" {
        *function = transmute::<pfn::DestroyActionSet, _>(destroy_action_set);
    } else if name == b"xrDestroySwapchain" {
        *function = transmute::<pfn::DestroySwapchain, _>(destroy_swapchain);
    } else if name == b"xrDestroySession" {
        *function = transmute::<pfn::DestroySession, _>(destroy_session);
    } else if name == b"xrDestroyInstance" {
        *function = transmute::<pfn::DestroyInstance, _>(destroy_instance);
    } else {
        let _name = String::from_utf8_unchecked(name.to_vec());
        // eprintln!("[HOTHAM_SIMULATOR] {} is unimplemented", name);
        unsafe extern "system" fn bang() -> Result {
            panic!("BAD BADB ADB ADB BAD");
        }
        *function = transmute::<DummyFn, _>(bang);
        // return Result::ERROR_HANDLE_INVALID.into_raw();
    }
    Result::SUCCESS.into_raw()
}

static GET_INSTANCE_PROC_ADDR: PFN_xrGetInstanceProcAddr = Some(get_instance_proc_addr);

#[no_mangle]
pub unsafe extern "system" fn xrNegotiateLoaderRuntimeInterface(
    _loader_info: *const openxr_loader::XrNegotiateLoaderInfo,
    runtime_request: *mut openxr_loader::XrNegotiateRuntimeRequest,
) -> i32 {
    let runtime_request = &mut *runtime_request;
    runtime_request.runtimeInterfaceVersion = 1;
    runtime_request.runtimeApiVersion = openxr_sys::CURRENT_API_VERSION.into_raw();
    runtime_request.getInstanceProcAddr = GET_INSTANCE_PROC_ADDR;

    Result::SUCCESS.into_raw()
}

fn str_to_fixed_bytes(string: &'static str) -> [i8; 128] {
    let mut name = [0 as i8; 128];
    string
        .bytes()
        .zip(name.iter_mut())
        .for_each(|(b, ptr)| *ptr = b as i8);
    name
}

pub struct SwapChainSupportDetails {
    pub capabilities: vk::SurfaceCapabilitiesKHR,
    pub surface_formats: Vec<vk::SurfaceFormatKHR>,
    pub present_modes: Vec<vk::PresentModeKHR>,
}

impl SwapChainSupportDetails {
    pub fn query_swap_chain_support(
        entry: &AshEntry,
        instance: &AshInstance,
        physical_device: vk::PhysicalDevice,
        surface: vk::SurfaceKHR,
        queue_family_index: u32,
    ) -> SwapChainSupportDetails {
        let surface_ext = khr::Surface::new(entry, instance);
        let capabilities = unsafe {
            surface_ext
                .get_physical_device_surface_capabilities(physical_device, surface)
                .expect("unable to get capabilities")
        };
        let surface_formats = unsafe {
            surface_ext
                .get_physical_device_surface_formats(physical_device, surface)
                .expect("unable to get surface formats")
        };
        let present_modes = unsafe {
            surface_ext
                .get_physical_device_surface_present_modes(physical_device, surface)
                .expect("unable to get present modes")
        };

        let support = unsafe {
            surface_ext.get_physical_device_surface_support(
                physical_device,
                queue_family_index,
                surface,
            )
        }
        .expect("Unable to get surface support");
        assert!(support, "This device does not support a surface!");

        SwapChainSupportDetails {
            capabilities,
            surface_formats,
            present_modes,
        }
    }
}

pub fn transition_image_layout(
    device: &Device,
    queue: vk::Queue,
    command_pool: vk::CommandPool,
    image: vk::Image,
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
) {
    println!("[HOTHAM_SIMULATOR] Transitioning image {:?}", image);
    let command_buffer = begin_single_time_commands(device, command_pool);
    let subresource_range = vk::ImageSubresourceRange::builder()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .base_mip_level(0)
        .level_count(1)
        .base_array_layer(0)
        .layer_count(2)
        .build();

    let (src_access_mask, dst_access_mask, src_stage, dst_stage) =
        get_stage(old_layout, new_layout);

    let barrier = vk::ImageMemoryBarrier::builder()
        .old_layout(old_layout)
        .new_layout(new_layout)
        .src_access_mask(src_access_mask)
        .dst_access_mask(dst_access_mask)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .subresource_range(subresource_range)
        .image(image)
        .build();

    let dependency_flags = vk::DependencyFlags::empty();
    let image_memory_barriers = &[barrier];

    unsafe {
        device.cmd_pipeline_barrier(
            command_buffer,
            src_stage,
            dst_stage,
            dependency_flags,
            &[],
            &[],
            image_memory_barriers,
        )
    };
    end_single_time_commands(device, queue, command_buffer, command_pool);
    println!("[HOTHAM_SIMULATOR] Done transitioning image {:?}", image);
}

pub fn begin_single_time_commands(
    device: &Device,
    command_pool: vk::CommandPool,
) -> vk::CommandBuffer {
    let alloc_info = vk::CommandBufferAllocateInfo::builder()
        .command_buffer_count(1)
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_pool(command_pool);

    let command_buffer = unsafe {
        device
            .allocate_command_buffers(&alloc_info)
            .map(|mut b| b.pop().unwrap())
            .expect("Unable to allocate command buffer")
    };

    let begin_info =
        vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

    unsafe {
        device
            .begin_command_buffer(command_buffer, &begin_info)
            .expect("Unable to begin command buffer")
    }

    command_buffer
}

pub fn end_single_time_commands(
    device: &Device,
    queue: vk::Queue,
    command_buffer: vk::CommandBuffer,
    command_pool: vk::CommandPool,
) {
    unsafe {
        device
            .end_command_buffer(command_buffer)
            .expect("Unable to end command buffer");
    }

    let command_buffers = &[command_buffer];

    let submit_info = vk::SubmitInfo::builder()
        .command_buffers(command_buffers)
        .build();

    let submit_info = &[submit_info];

    unsafe {
        device
            .queue_submit(queue, submit_info, vk::Fence::null())
            .expect("Unable to submit to queue");
        device.queue_wait_idle(queue).expect("Unable to wait idle");
        device.free_command_buffers(command_pool, command_buffers)
    }
}

fn get_stage(
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
) -> (
    vk::AccessFlags,
    vk::AccessFlags,
    vk::PipelineStageFlags,
    vk::PipelineStageFlags,
) {
    if old_layout == vk::ImageLayout::UNDEFINED
        && new_layout == vk::ImageLayout::TRANSFER_DST_OPTIMAL
    {
        return (
            vk::AccessFlags::empty(),
            vk::AccessFlags::TRANSFER_WRITE,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::TRANSFER,
        );
    }

    if old_layout == vk::ImageLayout::TRANSFER_DST_OPTIMAL
        && new_layout == vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
    {
        return (
            vk::AccessFlags::TRANSFER_WRITE,
            vk::AccessFlags::SHADER_READ,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
        );
    }

    if old_layout == vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL
        && new_layout == vk::ImageLayout::PRESENT_SRC_KHR
    {
        return (
            vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            vk::AccessFlags::COLOR_ATTACHMENT_READ,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        );
    }

    if old_layout == vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL
        && new_layout == vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
    {
        return (
            vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            vk::AccessFlags::SHADER_READ,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
        );
    }

    panic!("Invalid layout transition!");
}

pub fn find_memory_type(
    instance: &AshInstance,
    physical_device: vk::PhysicalDevice,
    type_filter: u32,
    properties: vk::MemoryPropertyFlags,
) -> u32 {
    let device_memory_properties =
        unsafe { instance.get_physical_device_memory_properties(physical_device) };
    for i in 0..device_memory_properties.memory_type_count {
        let has_type = type_filter & (1 << i) != 0;
        let has_properties = device_memory_properties.memory_types[i as usize]
            .property_flags
            .contains(properties);
        if has_type && has_properties {
            return i;
        }
    }

    panic!("Unable to find suitable memory type")
}
