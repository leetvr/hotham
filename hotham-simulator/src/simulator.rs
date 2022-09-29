#![allow(
    non_snake_case,
    dead_code,
    non_upper_case_globals,
    non_camel_case_types
)]
use crate::openxr_loader::{self, XrExtensionProperties, XrResult};
use crate::space_state::SpaceState;
use crate::state::State;

use ash::{
    extensions::khr,
    util::read_spv,
    vk::{self, DeviceCreateInfo, Handle},
    Device, Entry as AshEntry, Instance as AshInstance,
};
use lazy_static::lazy_static;
use openxr_sys::GraphicsBindingVulkanKHR;
use openxr_sys::{
    platform::{VkDevice, VkInstance, VkPhysicalDevice, VkResult},
    Action, ActionCreateInfo, ActionSet, ActionSetCreateInfo, ActionSpaceCreateInfo,
    ActionStateBoolean, ActionStateFloat, ActionStateGetInfo, ActionStatePose, ActionsSyncInfo,
    Duration, EnvironmentBlendMode, EventDataBuffer, EventDataSessionStateChanged, Fovf,
    FrameBeginInfo, FrameEndInfo, FrameState, FrameWaitInfo, GraphicsRequirementsVulkanKHR,
    HapticActionInfo, HapticBaseHeader, Instance, InstanceCreateInfo, InstanceProperties,
    InteractionProfileSuggestedBinding, Path, Posef, Quaternionf, ReferenceSpaceCreateInfo,
    ReferenceSpaceType, Result, Session, SessionActionSetsAttachInfo, SessionBeginInfo,
    SessionCreateInfo, SessionState, Space, SpaceLocation, SpaceLocationFlags, StructureType,
    Swapchain, SwapchainCreateInfo, SwapchainImageAcquireInfo, SwapchainImageBaseHeader,
    SwapchainImageReleaseInfo, SwapchainImageVulkanKHR, SwapchainImageWaitInfo, SystemGetInfo,
    SystemId, SystemProperties, Time, Vector3f, Version, View, ViewConfigurationType,
    ViewConfigurationView, ViewLocateInfo, ViewState, ViewStateFlags, VulkanDeviceCreateInfoKHR,
    VulkanGraphicsDeviceGetInfoKHR, VulkanInstanceCreateInfoKHR, FALSE, TRUE,
};
use rand::random;
use std::{
    ffi::{CStr, CString},
    fmt::Debug,
    intrinsics::{copy_nonoverlapping, transmute},
    io::Cursor,
    mem::size_of,
    os::raw::c_char,
    ptr::{self, null_mut},
    slice,
    sync::{atomic::Ordering::Relaxed, mpsc::channel, Mutex, MutexGuard},
    thread,
};
use winit::event::DeviceEvent;

#[cfg(any(target_os = "windows", target_os = "linux"))]
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::run_return::EventLoopExtRunReturn,
    window::WindowBuilder,
};

#[cfg(target_os = "windows")]
use winit::platform::windows::EventLoopExtWindows;

#[cfg(target_os = "linux")]
use winit::platform::unix::EventLoopExtUnix;

static SWAPCHAIN_COLOR_FORMAT: vk::Format = vk::Format::B8G8R8A8_SRGB;
pub const NUM_VIEWS: usize = 2; // TODO: Make dynamic
pub const VIEWPORT_HEIGHT: u32 = 1000;
pub const VIEWPORT_WIDTH: u32 = 1000;
pub const CAMERA_FIELD_OF_VIEW: f32 = 1.; // about 57 degrees

lazy_static! {
    static ref STATE: Mutex<State> = Default::default();
}

#[derive(Debug, Clone, Default)]
struct HothamSession {
    test: usize,
}

#[no_mangle]
pub unsafe extern "C" fn enumerate_instance_extension_properties(
    _layerName: *const ::std::os::raw::c_char,
    propertyCapacityInput: u32,
    propertyCountOutput: *mut u32,
    properties: *mut XrExtensionProperties,
) -> XrResult {
    if propertyCapacityInput == 0 {
        *propertyCountOutput = 2;
        return Result::SUCCESS.into_raw();
    }

    let extension = "XR_KHR_vulkan_enable2";
    let name = str_to_fixed_bytes(extension);
    let extensions = std::ptr::slice_from_raw_parts_mut(properties, 2);
    (*extensions)[0] = openxr_loader::XrExtensionProperties {
        type_: StructureType::EXTENSION_PROPERTIES.into_raw(),
        next: ptr::null_mut(),
        extensionName: name,
        extensionVersion: 2,
    };
    let extension = "XR_KHR_vulkan_enable";
    let name = str_to_fixed_bytes(extension);
    (*extensions)[1] = openxr_loader::XrExtensionProperties {
        type_: StructureType::EXTENSION_PROPERTIES.into_raw(),
        next: ptr::null_mut(),
        extensionName: name,
        extensionVersion: 1,
    };
    Result::SUCCESS.into_raw()
}

#[no_mangle]
pub unsafe extern "system" fn create_instance(
    _create_info: *const InstanceCreateInfo,
    instance: *mut Instance,
) -> Result {
    *instance = Instance::from_raw(42);

    Result::SUCCESS
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
pub unsafe extern "system" fn create_vulkan_instance(
    _instance: Instance,
    create_info: *const VulkanInstanceCreateInfoKHR,
    vulkan_instance: *mut VkInstance,
    vulkan_result: *mut VkResult,
) -> Result {
    let vulkan_create_info: &ash::vk::InstanceCreateInfo =
        transmute(&(*create_info).vulkan_create_info);
    let get_instance_proc_addr = (*create_info).pfn_get_instance_proc_addr.unwrap();
    let vk_create_instance = CStr::from_bytes_with_nul_unchecked(b"vkCreateInstance\0").as_ptr();
    let create_instance: vk::PFN_vkCreateInstance =
        transmute(get_instance_proc_addr(ptr::null(), vk_create_instance));
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
    for ext in xr_extensions {
        enabled_extensions.push(CStr::from_ptr(*ext));
    }

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
        get_instance_proc_addr: transmute(get_instance_proc_addr),
    };
    let ash_instance = AshInstance::load(&static_fn, instance);

    *vulkan_instance = transmute(instance);

    let mut state = STATE.lock().unwrap();

    state.vulkan_entry.replace(entry);
    state.vulkan_instance.replace(ash_instance);
    Result::SUCCESS
}

pub unsafe extern "system" fn create_vulkan_device(
    _instance: Instance,
    create_info: *const VulkanDeviceCreateInfoKHR,
    vulkan_device: *mut VkDevice,
    vulkan_result: *mut VkResult,
) -> Result {
    *vulkan_result = ash::vk::Result::SUCCESS.as_raw();

    #[allow(clippy::transmute_ptr_to_ref)] // TODO We shouldn't get a `&mut` from a `*const`.
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
    *vulkan_device = transmute(device.handle());
    let queue_family_index =
        slice::from_raw_parts(create_info.p_queue_create_infos, 1)[0].queue_family_index;

    create_and_store_device(device, queue_family_index, &mut state);

    Result::SUCCESS
}

unsafe fn create_and_store_device(device: ash::Device, queue_family_index: u32, state: &mut State) {
    let info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);
    state.swapchain_fence = device.create_fence(&info, None).unwrap();
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
    state.device = Some(device);

    println!(
        "[HOTHAM_SIMULATOR] Done! Device created: {:?}",
        state.device.as_ref().unwrap().handle()
    );
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

pub unsafe extern "system" fn create_vulkan_physical_device(
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

pub unsafe extern "system" fn get_vulkan_physical_device(
    _instance: Instance,
    _system_id: SystemId,
    vk_instance: VkInstance,
    vk_physical_device: *mut VkPhysicalDevice,
) -> Result {
    // Create an entry
    let entry = AshEntry::new().unwrap();

    // Create an instance wrapping the instance we were passed
    let ash_instance = AshInstance::load(entry.static_fn(), transmute(vk_instance));

    // Create the device and assign it
    let physical_device = ash_instance
        .enumerate_physical_devices()
        .unwrap()
        .pop()
        .unwrap();

    println!(
        "[HOTHAM_SIMULATOR] Created physical device: {:?}",
        physical_device
    );
    *vk_physical_device = transmute(physical_device);

    // Store everything in state.
    let mut state = STATE.lock().unwrap();
    state.vulkan_entry = Some(entry);
    state.vulkan_instance = Some(ash_instance);
    state.physical_device = physical_device;

    Result::SUCCESS
}

pub unsafe extern "system" fn get_vulkan_graphics_requirements(
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

// #[cfg(target_os = "windows")]
pub unsafe extern "system" fn get_instance_properties(
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

pub unsafe extern "system" fn enumerate_environment_blend_modes(
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

pub unsafe extern "system" fn get_system(
    _instance: Instance,
    _get_info: *const SystemGetInfo,
    system_id: *mut SystemId,
) -> Result {
    *system_id = SystemId::from_raw(42);
    Result::SUCCESS
}

pub unsafe extern "system" fn create_session(
    _instance: Instance,
    create_info: *const SessionCreateInfo,
    session: *mut Session,
) -> Result {
    *session = Session::from_raw(42);
    let mut state = STATE.lock().unwrap();
    if state.device.is_none() {
        let graphics_binding = &*((*create_info).next as *const GraphicsBindingVulkanKHR);
        let vk_device = graphics_binding.device;
        let instance = state.vulkan_instance.as_ref().unwrap();
        let device = ash::Device::load(instance.fp_v1_0(), transmute(vk_device));
        let queue_family_index = graphics_binding.queue_family_index;
        create_and_store_device(device, queue_family_index, &mut state);
    }

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
        .format(SWAPCHAIN_COLOR_FORMAT)
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
        width: VIEWPORT_WIDTH as _,
        height: VIEWPORT_HEIGHT as _,
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

        let viewport = vk::Viewport::builder()
            .height(extent.height as _)
            // .width((extent.width / (NUM_VIEWS as u32)) as _)
            .width(extent.width as _)
            .max_depth(1.)
            .min_depth(0.)
            .build();
        let scissor = vk::Rect2D {
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
        // if NUM_VIEWS > 1 {
        //     viewport.x = extent.width as f32 / 2.0;
        //     scissor.offset.x = extent.width as i32 / 2;
        //     device.cmd_set_viewport(*command_buffer, 0, &[viewport]);
        //     device.cmd_set_scissor(*command_buffer, 0, &[scissor]);

        //     device.cmd_bind_pipeline(*command_buffer, pipeline_bind_point, pipelines[1]);
        //     device.cmd_draw(*command_buffer, 3, 1, 0, 0);
        // }

        device.cmd_end_render_pass(*command_buffer);
        device
            .end_command_buffer(*command_buffer)
            .expect("Unable to end command buffer");
    }
    command_buffers
}

pub unsafe extern "system" fn create_action_set(
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

pub unsafe extern "system" fn create_action(
    _action_set: ActionSet,
    _create_info: *const ActionCreateInfo,
    action_out: *mut Action,
) -> Result {
    *action_out = Action::from_raw(random());
    Result::SUCCESS
}

pub unsafe extern "system" fn suggest_interaction_profile_bindings(
    _instance: Instance,
    suggested_bindings: *const InteractionProfileSuggestedBinding,
) -> Result {
    let suggested_bindings = *suggested_bindings;
    let mut state = STATE.lock().unwrap();

    let bindings = std::slice::from_raw_parts(
        suggested_bindings.suggested_bindings,
        suggested_bindings.count_suggested_bindings as _,
    );

    for binding in bindings {
        state
            .action_state
            .add_binding(binding.binding, binding.action);
    }

    Result::SUCCESS
}

pub unsafe extern "system" fn string_to_path(
    _instance: Instance,
    path_string: *const c_char,
    path_out: *mut Path,
) -> Result {
    match CStr::from_ptr(path_string).to_str() {
        Ok(s) => {
            let path = Path::from_raw(rand::random());
            println!(
                "[HOTHAM_SIMULATOR] Created path {:?} for {}",
                path_string, s
            );
            STATE
                .lock()
                .unwrap()
                .path_string
                .insert(path, s.to_string());
            STATE
                .lock()
                .unwrap()
                .string_path
                .insert(s.to_string(), path);
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
    println!("[HOTHAM_SIMULATOR] Attach action sets called");
    Result::SUCCESS
}

// TODO: Handle aim pose.
pub unsafe extern "system" fn create_action_space(
    _session: Session,
    create_info: *const ActionSpaceCreateInfo,
    space_out: *mut Space,
) -> Result {
    let mut state = STATE.lock().unwrap();
    let raw = random();
    let space = Space::from_raw(raw);

    match state
        .path_string
        .get(&(*create_info).subaction_path)
        .map(|s| s.as_str())
    {
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
            println!(
                "[HOTHAM_SIMULATOR] Created left hand space: {:?}, {:?}",
                space_state, space
            );
            state.left_hand_space = raw;
            state.spaces.insert(raw, space_state);
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
            println!(
                "[HOTHAM_SIMULATOR] Created right hand space: {:?}, {:?}",
                space_state, space
            );
            state.right_hand_space = raw;
            state.spaces.insert(raw, space_state);
        }
        Some(path) => {
            let space_state = SpaceState::new(path);
            println!("[HOTHAM_SIMULATOR] Created space for path: {}", path);
            state.spaces.insert(raw, space_state);
        }
        _ => {}
    }

    *space_out = space;
    Result::SUCCESS
}

pub unsafe extern "system" fn create_reference_space(
    _session: Session,
    create_info: *const ReferenceSpaceCreateInfo,
    out_space: *mut Space,
) -> Result {
    let mut state = STATE.lock().unwrap();
    let reference_space;
    let create_info = *create_info;

    // Our "reference space" is Stage with no rotation
    if create_info.reference_space_type == ReferenceSpaceType::STAGE
        && create_info.pose_in_reference_space.orientation.w != 1.0
    {
        // Magic value
        reference_space = Space::from_raw(0);
        println!(
            "[HOTHAM_SIMULATOR] Stage reference space created: {:?}",
            reference_space
        );
        state.reference_space = reference_space;
    } else {
        reference_space = Space::from_raw(random());
    }

    let mut space_state = SpaceState::new("Reference Space");
    space_state.position = create_info.pose_in_reference_space.position;
    space_state.orientation = create_info.pose_in_reference_space.orientation;

    state.spaces.insert(reference_space.into_raw(), space_state);

    *out_space = reference_space;
    Result::SUCCESS
}

pub unsafe extern "system" fn poll_event(
    _instance: Instance,
    event_data: *mut EventDataBuffer,
) -> Result {
    let mut state = STATE.lock().unwrap();
    let mut next_state = state.session_state;
    if state.session_state == SessionState::UNKNOWN {
        next_state = SessionState::IDLE;
        state.has_event = true;
    }
    if state.session_state == SessionState::IDLE {
        next_state = SessionState::READY;
        state.has_event = true;
    }
    if state.session_state == SessionState::READY {
        next_state = SessionState::SYNCHRONIZED;
        state.has_event = true;
    }
    if state.session_state == SessionState::SYNCHRONIZED {
        next_state = SessionState::VISIBLE;
        state.has_event = true;
    }
    if state.session_state == SessionState::SYNCHRONIZED {
        next_state = SessionState::FOCUSED;
        state.has_event = true;
    }

    if state.has_event {
        let data = EventDataSessionStateChanged {
            ty: StructureType::EVENT_DATA_SESSION_STATE_CHANGED,
            next: ptr::null(),
            session: Session::from_raw(42),
            state: next_state,
            time: openxr_sys::Time::from_nanos(10),
        };
        copy_nonoverlapping(&data, transmute(event_data), 1);
        state.has_event = false;
        state.session_state = next_state;

        Result::SUCCESS
    } else {
        Result::EVENT_UNAVAILABLE
    }
}

pub unsafe extern "system" fn begin_session(
    session: Session,
    _begin_info: *const SessionBeginInfo,
) -> Result {
    println!("[HOTHAM_SIMULATOR] Beginning session: {:?}", session);
    Result::SUCCESS
}
pub unsafe extern "system" fn wait_frame(
    _session: Session,
    _frame_wait_info: *const FrameWaitInfo,
    frame_state: *mut FrameState,
) -> Result {
    let state = STATE.lock().unwrap();
    let should_render = state.session_state == SessionState::VISIBLE
        || state.session_state == SessionState::FOCUSED;

    *frame_state = FrameState {
        ty: StructureType::FRAME_STATE,
        next: ptr::null_mut(),
        predicted_display_time: Time::from_nanos(1),
        predicted_display_period: Duration::from_nanos(1),
        should_render: should_render.into(),
    };
    Result::SUCCESS
}

pub unsafe extern "system" fn begin_frame(
    _session: Session,
    _frame_begin_info: *const FrameBeginInfo,
) -> Result {
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
    if view_capacity_input == 0 {
        *view_count_output = NUM_VIEWS as _;
        return Result::SUCCESS;
    }

    println!(
        "[HOTHAM_SIMULATOR] enumerate_view_configuration_views called with: {}",
        view_capacity_input
    );

    let views = std::ptr::slice_from_raw_parts_mut(views, NUM_VIEWS);

    for i in 0..NUM_VIEWS {
        (*views)[i] = ViewConfigurationView {
            ty: StructureType::VIEW_CONFIGURATION_VIEW,
            next: null_mut(),
            recommended_image_rect_width: VIEWPORT_WIDTH as _,
            max_image_rect_width: VIEWPORT_WIDTH as _,
            recommended_image_rect_height: VIEWPORT_HEIGHT as _,
            max_image_rect_height: VIEWPORT_HEIGHT as _,
            recommended_swapchain_sample_count: 3,
            max_swapchain_sample_count: 3,
        };
    }
    Result::SUCCESS
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
pub unsafe extern "system" fn create_xr_swapchain(
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
                .layer_count(NUM_VIEWS as _)
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

#[cfg(any(target_os = "windows", target_os = "linux"))]
unsafe fn build_swapchain(state: &mut MutexGuard<State>) -> vk::SwapchainKHR {
    use winit::event::{ElementState, MouseButton};

    let entry = state.vulkan_entry.as_ref().unwrap().clone();
    let instance = state.vulkan_instance.as_ref().unwrap().clone();
    let device = state.device.as_ref().unwrap();
    let physical_device = state.physical_device;
    let swapchain_ext = khr::Swapchain::new(&instance, device);
    let queue_family_index = state.present_queue_family_index;
    let close_window = state.close_window.clone();

    let (swapchain_tx, swapchain_rx) = channel();
    let (mouse_event_tx, mouse_event_rx) = channel();
    let (keyboard_event_tx, keyboard_event_rx) = channel();
    let window_thread_handle = thread::spawn(move || {
        let mut event_loop: EventLoop<()> = EventLoop::new_any_thread();
        let visible = true;
        println!(
            "[HOTHAM_SIMULATOR] Creating window with visible {}..",
            visible
        );
        let window = WindowBuilder::new()
            .with_inner_size(PhysicalSize::new(VIEWPORT_WIDTH, VIEWPORT_HEIGHT))
            .with_title("Hotham Simulator")
            .with_visible(visible)
            // .with_drag_and_drop(false)
            .build(&event_loop)
            .unwrap();
        println!("WINDOW SCALE FACTOR, {:?}", window.scale_factor());
        println!("[HOTHAM_SIMULATOR] ..done.");
        let extent = vk::Extent2D {
            height: VIEWPORT_HEIGHT,
            width: VIEWPORT_WIDTH,
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
            .image_format(SWAPCHAIN_COLOR_FORMAT)
            .image_color_space(vk::ColorSpaceKHR::SRGB_NONLINEAR)
            .image_array_layers(1)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .image_extent(extent)
            .queue_family_indices(&[])
            .pre_transform(swapchain_support_details.capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(vk::PresentModeKHR::IMMEDIATE)
            .clipped(true)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT);

        println!("[HOTHAM_SIMULATOR] About to create swapchain..");
        let swapchain = swapchain_ext.create_swapchain(&create_info, None).unwrap();
        println!(
            "[HOTHAM_SIMULATOR] Created swapchain: {:?}. Sending..",
            swapchain
        );
        swapchain_tx.send((surface, swapchain)).unwrap();

        if !visible {
            return;
        }
        let cl2 = close_window.clone();

        let mut mouse_pressed = false;

        event_loop.run_return(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            if close_window.load(Relaxed) {
                println!("[HOTHAM_SIMULATOR] Closed called!");
                *control_flow = ControlFlow::Exit;
            }

            match event {
                Event::WindowEvent { event, window_id } => match event {
                    WindowEvent::CloseRequested => {
                        if window_id == window.id() {
                            *control_flow = ControlFlow::Exit
                        }
                    }
                    WindowEvent::KeyboardInput { input, .. } => {
                        keyboard_event_tx.send(input).unwrap()
                    }
                    WindowEvent::MouseInput {
                        button: MouseButton::Left,
                        state,
                        ..
                    } => {
                        mouse_pressed = state == ElementState::Pressed;
                    }
                    _ => {}
                },
                Event::LoopDestroyed => {}
                Event::MainEventsCleared => {
                    window.request_redraw();
                }
                Event::RedrawRequested(_window_id) => {}

                Event::DeviceEvent { event, .. } => {
                    if mouse_pressed {
                        if let DeviceEvent::MouseMotion { delta } = event {
                            mouse_event_tx.send(delta).unwrap();
                        }
                    }
                }
                _ => (),
            }
        });

        cl2.store(true, Relaxed);
    });
    let (surface, swapchain) = swapchain_rx.recv().unwrap();

    println!("[HOTHAM_SIMULATOR] Received swapchain: {:?}", swapchain);
    let instance = state.vulkan_instance.as_ref().unwrap().clone();
    let swapchain_ext = khr::Swapchain::new(&instance, device);

    state.mouse_event_rx = Some(mouse_event_rx);
    state.keyboard_event_rx = Some(keyboard_event_rx);
    state.surface = surface;
    state.window_thread_handle = Some(window_thread_handle);
    state.internal_swapchain = swapchain;
    state.internal_swapchain_images = swapchain_ext
        .get_swapchain_images(swapchain)
        .expect("Unable to get swapchain images");
    state.internal_swapchain_image_views = create_swapchain_image_views(state);

    println!("[HOTHAM_SIMULATOR] Creating descriptor sets..");
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
                    .descriptor_count(9)
                    .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .build()])
                .max_sets(9)
                .build(),
            None,
        )
        .expect("Unable to create descriptor pool");

    println!(
        "[HOTHAM_SIMULATOR] Created descriptor pool {:?}",
        descriptor_pool
    );

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

    println!(
        "[HOTHAM_SIMULATOR] Allocating descriptor sets with layouts: {:?}",
        set_layouts
    );

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
                .format(SWAPCHAIN_COLOR_FORMAT)
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
                .width(VIEWPORT_WIDTH)
                .height(VIEWPORT_HEIGHT)
                .layers(1);

            unsafe { device.create_framebuffer(&create_info, None).unwrap() }
        })
        .collect::<Vec<_>>()
}

pub unsafe extern "system" fn enumerate_swapchain_images(
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
        .array_layers(NUM_VIEWS as _)
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

    for image in &images {
        transition_image_layout(
            device,
            state.present_queue,
            state.command_pool,
            *image,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        );
    }

    (images, device_memory)
}

pub unsafe extern "system" fn acquire_swapchain_image(
    swapchain: Swapchain,
    _acquire_info: *const SwapchainImageAcquireInfo,
    index: *mut u32,
) -> Result {
    let mut state = STATE.lock().unwrap();
    let swapchain = vk::SwapchainKHR::from_raw(swapchain.into_raw());
    let device = state.device.as_ref().unwrap();
    let ext = khr::Swapchain::new(state.vulkan_instance.as_ref().unwrap(), device);
    let fence = state.swapchain_fence;

    device.wait_for_fences(&[fence], true, u64::MAX).unwrap();
    device.reset_fences(&[fence]).unwrap();

    let (i, _) = ext
        .acquire_next_image(swapchain, u64::MAX - 1, vk::Semaphore::null(), fence)
        .unwrap();

    *index = i;

    state.image_index = *index;
    Result::SUCCESS
}

pub unsafe extern "system" fn wait_swapchain_image(
    _swapchain: Swapchain,
    _wait_info: *const SwapchainImageWaitInfo,
) -> Result {
    Result::SUCCESS
}

pub unsafe extern "system" fn dummy() -> Result {
    println!("[HOTHAM_SIMULATOR] Uh oh, dummy called");
    Result::SUCCESS
}

pub unsafe extern "system" fn locate_space(
    space: Space,
    _base_space: Space,
    _time: Time,
    location_out: *mut SpaceLocation,
) -> Result {
    match STATE.lock().unwrap().spaces.get(&space.into_raw()) {
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
    *state = ActionStatePose {
        ty: StructureType::ACTION_STATE_POSE,
        next: null_mut(),
        is_active: TRUE,
    };
    Result::SUCCESS
}

pub unsafe extern "system" fn sync_actions(
    _session: Session,
    _sync_info: *const ActionsSyncInfo,
) -> Result {
    let mut state = STATE.lock().unwrap();
    state.update_camera_rotation();
    state.update_camera_position();
    state.update_action_state();

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
    *view_count_output = NUM_VIEWS as _;

    if view_capacity_input == 0 {
        return Result::SUCCESS;
    }

    *view_state = ViewState {
        ty: StructureType::VIEW_STATE,
        next: null_mut(),
        view_state_flags: ViewStateFlags::ORIENTATION_VALID | ViewStateFlags::POSITION_VALID,
    };
    let views = slice::from_raw_parts_mut(views, NUM_VIEWS);
    let state = STATE.lock().unwrap();
    #[allow(clippy::approx_constant)]
    for (i, view) in views.iter_mut().enumerate() {
        let pose = state.view_poses[i];

        // The actual fov is defined as (right - left). As these are all symetrical, we just divide the fov variable by 2.
        *view = View {
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
    }

    Result::SUCCESS
}

pub unsafe extern "system" fn release_swapchain_image(
    _swapchain: Swapchain,
    _release_info: *const SwapchainImageReleaseInfo,
) -> Result {
    Result::SUCCESS
}

pub unsafe extern "system" fn end_frame(
    _session: Session,
    frame_end_info: *const FrameEndInfo,
) -> Result {
    // If there are no layers to present, we're done.
    if (*frame_end_info).layer_count == 0 {
        return Result::SUCCESS;
    }
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

    match ext.queue_present(queue, &present_info) {
        Ok(_) => {
            transition_image_layout(
                device,
                queue,
                state.command_pool,
                image,
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            );
            state.frame_count += 1;
            Result::SUCCESS
        }
        Err(e) => {
            eprintln!("[HOTHAM_SIMULATOR] !ERROR RENDERING FRAME! {:?}", e);
            Result::ERROR_VALIDATION_FAILURE
        }
    }
}

pub unsafe extern "system" fn request_exit_session(_session: Session) -> Result {
    let mut state = STATE.lock().unwrap();
    state.session_state = SessionState::EXITING;
    state.has_event = true;
    Result::SUCCESS
}

pub unsafe extern "system" fn destroy_space(_space: Space) -> Result {
    Result::SUCCESS
}

pub unsafe extern "system" fn destroy_action(_action: Action) -> Result {
    Result::SUCCESS
}

pub unsafe extern "system" fn destroy_action_set(_action_set: ActionSet) -> Result {
    Result::SUCCESS
}

pub unsafe extern "system" fn destroy_swapchain(_swapchain: Swapchain) -> Result {
    Result::SUCCESS
}

pub unsafe extern "system" fn destroy_session(_session: Session) -> Result {
    Result::SUCCESS
}

pub unsafe extern "system" fn destroy_instance(_instance: Instance) -> Result {
    STATE.lock().unwrap().destroy();
    Result::SUCCESS
}

pub unsafe extern "system" fn enumerate_view_configurations(
    _instance: Instance,
    _system_id: SystemId,
    _view_configuration_type_capacity_input: u32,
    view_configuration_type_count_output: *mut u32,
    _view_configuration_types: *mut ViewConfigurationType,
) -> Result {
    *view_configuration_type_count_output = 0;
    Result::SUCCESS
}

pub unsafe extern "system" fn enumerate_reference_spaces(
    _session: Session,
    space_capacity_input: u32,
    space_count_output: *mut u32,
    spaces: *mut ReferenceSpaceType,
) -> Result {
    *space_count_output = 1;
    if space_capacity_input == 0 {
        return Result::SUCCESS;
    }

    let spaces = slice::from_raw_parts_mut(spaces, 1);
    spaces[0] = ReferenceSpaceType::STAGE;

    Result::SUCCESS
}

pub unsafe extern "system" fn get_system_properties(
    _instance: Instance,
    _system_id: SystemId,
    _properties: *mut SystemProperties,
) -> Result {
    Result::SUCCESS
}

pub unsafe extern "system" fn enumerate_swapchain_formats(
    _session: Session,
    format_capacity_input: u32,
    format_count_output: *mut u32,
    formats: *mut i64,
) -> Result {
    if format_capacity_input == 0 {
        *format_count_output = 1;
        return Result::SUCCESS;
    }

    *formats = SWAPCHAIN_COLOR_FORMAT.as_raw() as i64;

    Result::SUCCESS
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
    Result::SUCCESS
}

pub unsafe extern "system" fn end_session(_session: Session) -> Result {
    let mut state = STATE.lock().unwrap();

    state.session_state = SessionState::EXITING;
    state.has_event = true;
    Result::SUCCESS
}

pub unsafe extern "system" fn get_action_state_boolean(
    _session: Session,
    get_info: *const ActionStateGetInfo,
    action_state: *mut ActionStateBoolean,
) -> Result {
    let state = STATE.lock().unwrap();
    let action = (*get_info).action;
    let current_state = state.get_action_state_boolean(action);

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
    /* explicit no-op, could possibly be extended with controller support in future if winit ever
     * provides such APIs */
    Result::SUCCESS
}

pub unsafe extern "system" fn get_vulkan_instance_extensions(
    _instance: Instance,
    _system_id: SystemId,
    buffer_capacity_input: u32,
    buffer_count_output: *mut u32,
    buffer: *mut c_char,
) -> Result {
    let event_loop: EventLoop<()> = EventLoop::new_any_thread();
    let window = WindowBuilder::new()
        // .with_drag_and_drop(false)
        .with_visible(false)
        .build(&event_loop)
        .unwrap();
    let enabled_extensions = ash_window::enumerate_required_extensions(&window).unwrap();
    let extensions = enabled_extensions
        .iter()
        .map(|e| e.to_str().unwrap())
        .collect::<Vec<&str>>()
        .join(" ")
        .into_bytes();

    let length = extensions.len() + 1;

    if buffer_capacity_input == 0 {
        (*buffer_count_output) = length as _;
        return Result::SUCCESS;
    }

    let extensions = CString::from_vec_unchecked(extensions);

    let buffer = slice::from_raw_parts_mut(buffer, length);
    let bytes = extensions.as_bytes_with_nul();
    for i in 0..length {
        buffer[i] = bytes[i] as _;
    }

    Result::SUCCESS
}

pub unsafe extern "system" fn get_vulkan_device_extensions(
    _instance: Instance,
    _system_id: SystemId,
    buffer_capacity_input: u32,
    buffer_count_output: *mut u32,
    buffer: *mut c_char,
) -> Result {
    let extensions = khr::Swapchain::name();
    let bytes = extensions.to_bytes_with_nul();
    let length = bytes.len();
    if buffer_capacity_input == 0 {
        *buffer_count_output = length as _;
        return Result::SUCCESS;
    }

    let buffer = slice::from_raw_parts_mut(buffer, length);
    for i in 0..length {
        buffer[i] = bytes[i] as _;
    }

    Result::SUCCESS
}

fn str_to_fixed_bytes(string: &str) -> [i8; 128] {
    let mut name = [0i8; 128];
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
    // println!("[HOTHAM_SIMULATOR] Transitioning image {:?}", image);
    let command_buffer = begin_single_time_commands(device, command_pool);
    let subresource_range = vk::ImageSubresourceRange::builder()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .base_mip_level(0)
        .level_count(1)
        .base_array_layer(0)
        .layer_count(NUM_VIEWS as _)
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
    // println!("[HOTHAM_SIMULATOR] Done transitioning image {:?}", image);
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

    if old_layout == vk::ImageLayout::UNDEFINED
        && new_layout == vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL
    {
        return (
            vk::AccessFlags::empty(),
            vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
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

    if old_layout == vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
        && new_layout == vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL
    {
        return (
            vk::AccessFlags::SHADER_READ,
            vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
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
