mod camera;
mod input_context;

use anyhow::{bail, Result};
use ash::vk;

use glam::Vec2;
use hotham_editor_protocol::{responses, EditorServer, RequestType};
use lazy_vulkan::{
    find_memorytype_index, vulkan_context::VulkanContext, vulkan_texture::VulkanTexture, DrawCall,
    LazyRenderer, LazyVulkan, SwapchainInfo, Vertex,
};
use log::{debug, info, trace};

#[cfg(not(target_os = "windows"))]
use std::os::unix::net::{UnixListener, UnixStream};
use std::time::Instant;
#[cfg(target_os = "windows")]
use uds_windows::{UnixListener, UnixStream};
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::ControlFlow,
    platform::run_return::EventLoopExtRunReturn,
};

use crate::camera::Camera;

/// Compile your own damn shaders! LazyVulkan is just as lazy as you are!
static FRAGMENT_SHADER: &'_ [u8] = include_bytes!("shaders/triangle.frag.spv");
static VERTEX_SHADER: &'_ [u8] = include_bytes!("shaders/triangle.vert.spv");
const SWAPCHAIN_FORMAT: vk::Format = vk::Format::R8G8B8A8_SRGB; // OpenXR really, really wants us to use SRGB swapchains
static UNIX_SOCKET_PATH: &'_ str = "hotham_editor.socket";

pub fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    // Oh, you thought you could supply your own Vertex type? What is this, a rendergraph?!
    // Better make sure those shaders use the right layout!
    // **LAUGHS IN VULKAN**
    let vertices = [
        Vertex::new([1.0, 1.0, 0.0, 1.0], [1.0, 1.0, 1.0, 0.0], [1.0, 1.0]), // bottom right
        Vertex::new([-1.0, 1.0, 0.0, 1.0], [1.0, 1.0, 1.0, 0.0], [0.0, 1.0]), // bottom left
        Vertex::new([1.0, -1.0, 0.0, 1.0], [1.0, 1.0, 1.0, 0.0], [1.0, 0.0]), // top right
        Vertex::new([-1.0, -1.0, 0.0, 1.0], [1.0, 1.0, 1.0, 0.0], [0.0, 0.0]), // top left
    ];

    // Your own index type?! What are you going to use, `u16`?
    let indices = [0, 1, 2, 2, 1, 3];

    // Alright, let's build some stuff
    let (mut lazy_vulkan, mut lazy_renderer, mut event_loop) = LazyVulkan::builder()
        .initial_vertices(&vertices)
        .initial_indices(&indices)
        .fragment_shader(FRAGMENT_SHADER)
        .vertex_shader(VERTEX_SHADER)
        .with_present(true)
        .build();

    // Let's do something totally normal and wait for a TCP connection
    if std::fs::remove_file(UNIX_SOCKET_PATH).is_ok() {
        debug!("Removed pre-existing unix socket at {UNIX_SOCKET_PATH}");
    }

    let listener = UnixListener::bind(UNIX_SOCKET_PATH).unwrap();
    info!("Listening on {UNIX_SOCKET_PATH} - waiting for client..");
    let swapchain_info = SwapchainInfo {
        image_count: lazy_vulkan.surface.desired_image_count,
        resolution: lazy_vulkan.surface.surface_resolution,
        format: SWAPCHAIN_FORMAT,
    };
    let (stream, _) = listener.accept().unwrap();
    info!("Client connected! Doing OpenXR setup..");
    let mut server = EditorServer::new(stream);
    let xr_swapchain = do_openxr_setup(&mut server, lazy_vulkan.context(), &swapchain_info)?;
    info!("..done!");
    let textures = create_render_textures(
        lazy_vulkan.context(),
        &mut lazy_renderer,
        xr_swapchain.images,
    );

    let mut last_frame_time = Instant::now();
    let mut keyboard_events = Vec::new();
    let mut mouse_events = Vec::new();
    let mut camera = Camera::default();

    // Off we go!
    let mut winit_initializing = true;
    let mut focused = false;
    event_loop.run_return(|event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::WindowEvent {
                event:
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    },
                ..
            } => *control_flow = ControlFlow::Exit,

            Event::WindowEvent {
                event: WindowEvent::Focused(window_focused),
                ..
            } => {
                focused = window_focused;
            }

            Event::NewEvents(cause) => {
                if cause == winit::event::StartCause::Init {
                    winit_initializing = true;
                } else {
                    winit_initializing = false;
                }
            }

            Event::MainEventsCleared => {
                let framebuffer_index = lazy_vulkan.render_begin();
                camera.process_input(last_frame_time, &keyboard_events, &mouse_events);
                keyboard_events.clear();
                mouse_events.clear();

                check_request(&mut server, RequestType::LocateView).unwrap();
                server.send_response(&camera.as_pose()).unwrap();

                check_request(&mut server, RequestType::WaitFrame).unwrap();
                server.send_response(&0).unwrap();

                check_request(&mut server, RequestType::AcquireSwapchainImage).unwrap();
                server.send_response(&framebuffer_index).unwrap();

                check_request(&mut server, RequestType::LocateView).unwrap();
                server.send_response(&camera.as_pose()).unwrap();

                check_request(&mut server, RequestType::EndFrame).unwrap();
                server.send_response(&0).unwrap();

                let texture_id = textures[framebuffer_index as usize].id;
                lazy_renderer.render(
                    lazy_vulkan.context(),
                    framebuffer_index,
                    &[DrawCall::new(
                        0,
                        indices.len() as _,
                        texture_id,
                        lazy_vulkan::Workflow::Main,
                    )],
                );

                let semaphore = xr_swapchain.semaphores[framebuffer_index as usize];
                lazy_vulkan.render_end(
                    framebuffer_index,
                    &[semaphore, lazy_vulkan.rendering_complete_semaphore],
                );
                last_frame_time = Instant::now();
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                if !winit_initializing {
                    let new_render_surface = lazy_vulkan.resized(size.width, size.height);
                    lazy_renderer.update_surface(new_render_surface, &lazy_vulkan.context().device);
                }
            }
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput { input, .. },
                ..
            } => {
                if focused && input.virtual_keycode.is_some() {
                    keyboard_events.push(input);
                }
            }
            Event::DeviceEvent { event, .. } => match event {
                winit::event::DeviceEvent::MouseMotion { delta } => {
                    if focused {
                        mouse_events.push(MouseInput::MouseMoved(
                            [delta.0 as f32, delta.1 as f32].into(), // translate from screen space to world space.. sort of
                        ))
                    }
                }
                winit::event::DeviceEvent::Button {
                    button: 1,
                    state: ElementState::Pressed,
                } => {
                    if focused {
                        mouse_events.push(MouseInput::LeftClickPressed)
                    }
                }
                winit::event::DeviceEvent::Button {
                    button: 1,
                    state: ElementState::Released,
                } => {
                    if focused {
                        mouse_events.push(MouseInput::LeftClickReleased)
                    }
                }
                _ => {}
            },
            _ => (),
        }
    });

    // I guess we better do this or else the Dreaded Validation Layers will complain
    unsafe {
        lazy_renderer.cleanup(&lazy_vulkan.context().device);
    }

    Ok(())
}

pub enum MouseInput {
    LeftClickPressed,
    LeftClickReleased,
    MouseMoved(Vec2),
}

pub struct XrSwapchain {
    images: Vec<vk::Image>,
    semaphores: Vec<vk::Semaphore>,
}

fn do_openxr_setup(
    server: &mut EditorServer<UnixStream>,
    vulkan_context: &VulkanContext,
    swapchain_info: &SwapchainInfo,
) -> Result<XrSwapchain> {
    let (images, image_memory_handles) =
        unsafe { create_render_images(vulkan_context, &swapchain_info) };
    let (semaphores, semaphore_handles) =
        unsafe { create_semaphores(vulkan_context, swapchain_info.image_count) };

    check_request(server, RequestType::GetViewCount)?;
    server.send_response(&swapchain_info.image_count)?;

    check_request(server, RequestType::GetViewConfiguration)?;
    server.send_response(&responses::ViewConfiguration {
        width: swapchain_info.resolution.width,
        height: swapchain_info.resolution.height,
    })?;

    check_request(server, RequestType::GetSwapchainInfo)?;
    server.send_response(&responses::SwapchainInfo {
        format: swapchain_info.format,
        resolution: swapchain_info.resolution,
    })?;

    check_request(server, RequestType::GetSwapchainImages)?;
    server.send_response_vec(&image_memory_handles)?;

    check_request(server, RequestType::GetSwapchainSemaphores)?;
    server.send_response_vec(&semaphore_handles)?;

    Ok(XrSwapchain { images, semaphores })
}

fn check_request(
    server: &mut EditorServer<UnixStream>,
    expected_request_type: RequestType,
) -> Result<(), anyhow::Error> {
    let header = server.get_request_header()?;
    if header.request_type != expected_request_type {
        bail!("Invalid request type: {:?}!", header.request_type);
    }
    trace!("Received request from client {expected_request_type:?}");
    Ok(())
}

#[cfg(target_os = "windows")]
unsafe fn create_semaphores(
    context: &lazy_vulkan::vulkan_context::VulkanContext,
    image_count: u32,
) -> (Vec<vk::Semaphore>, Vec<vk::HANDLE>) {
    let device = &context.device;
    let external_semaphore =
        ash::extensions::khr::ExternalSemaphoreWin32::new(&context.instance, &context.device);
    let handle_type = vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_WIN32_KMT;
    (0..image_count)
        .map(|_| {
            let mut external_semaphore_info =
                vk::ExportSemaphoreCreateInfo::builder().handle_types(handle_type);
            let semaphore = device
                .create_semaphore(
                    &vk::SemaphoreCreateInfo::builder().push_next(&mut external_semaphore_info),
                    None,
                )
                .unwrap();

            let handle = external_semaphore
                .get_semaphore_win32_handle(
                    &vk::SemaphoreGetWin32HandleInfoKHR::builder()
                        .handle_type(handle_type)
                        .semaphore(semaphore),
                )
                .unwrap();

            (semaphore, handle)
        })
        .unzip()
}

#[cfg(not(target_os = "windows"))]
unsafe fn create_semaphores(
    context: &lazy_vulkan::vulkan_context::VulkanContext,
    image_count: u32,
) -> (Vec<vk::Semaphore>, Vec<i32>) {
    let device = &context.device;
    let external_semaphore =
        ash::extensions::khr::ExternalSemaphoreFd::new(&context.instance, &context.device);
    let handle_type = vk::ExternalSemaphoreHandleTypeFlags::OPAQUE_FD;
    (0..image_count)
        .map(|_| {
            let mut external_semaphore_info =
                vk::ExportSemaphoreCreateInfo::builder().handle_types(handle_type);
            let semaphore = device
                .create_semaphore(
                    &vk::SemaphoreCreateInfo::builder().push_next(&mut external_semaphore_info),
                    None,
                )
                .unwrap();

            let handle = external_semaphore
                .get_semaphore_fd(
                    &vk::SemaphoreGetFdInfoKHR::builder()
                        .handle_type(handle_type)
                        .semaphore(semaphore),
                )
                .unwrap();

            (semaphore, handle)
        })
        .unzip()
}

fn create_render_textures(
    vulkan_context: &lazy_vulkan::vulkan_context::VulkanContext,
    renderer: &mut LazyRenderer,
    mut images: Vec<vk::Image>,
) -> Vec<VulkanTexture> {
    let descriptors = &mut renderer.descriptors;
    let address_mode = vk::SamplerAddressMode::REPEAT;
    let filter = vk::Filter::LINEAR;
    images
        .drain(..)
        .map(|image| {
            let view = unsafe { vulkan_context.create_image_view(image, SWAPCHAIN_FORMAT) };
            let sampler = unsafe {
                vulkan_context
                    .device
                    .create_sampler(
                        &vk::SamplerCreateInfo::builder()
                            .address_mode_u(address_mode)
                            .address_mode_v(address_mode)
                            .address_mode_w(address_mode)
                            .mag_filter(filter)
                            .min_filter(filter),
                        None,
                    )
                    .unwrap()
            };

            let id =
                unsafe { descriptors.update_texture_descriptor_set(view, sampler, vulkan_context) };

            lazy_vulkan::vulkan_texture::VulkanTexture {
                image,
                memory: vk::DeviceMemory::null(), // todo
                sampler,
                view,
                id,
            }
        })
        .collect()
}

#[cfg(target_os = "windows")]
type HandleOrFd = vk::HANDLE;
#[cfg(not(target_os = "windows"))]
type HandleOrFd = i32;

unsafe fn create_render_images(
    context: &lazy_vulkan::vulkan_context::VulkanContext,
    swapchain_info: &SwapchainInfo,
) -> (Vec<vk::Image>, Vec<HandleOrFd>) {
    let device = &context.device;
    let SwapchainInfo {
        resolution,
        format,
        image_count,
    } = swapchain_info;
    let handle_type = vk::ExternalMemoryHandleTypeFlags::OPAQUE_WIN32_KMT;

    (0..(*image_count))
        .map(|_| {
            let mut handle_info =
                vk::ExternalMemoryImageCreateInfo::builder().handle_types(handle_type);
            let image = device
                .create_image(
                    &vk::ImageCreateInfo {
                        image_type: vk::ImageType::TYPE_2D,
                        format: *format,
                        extent: (*resolution).into(),
                        mip_levels: 1,
                        array_layers: 2,
                        samples: vk::SampleCountFlags::TYPE_1,
                        tiling: vk::ImageTiling::OPTIMAL,
                        usage: vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
                        sharing_mode: vk::SharingMode::EXCLUSIVE,
                        p_next: &mut handle_info as *mut _ as *mut _,
                        ..Default::default()
                    },
                    None,
                )
                .unwrap();

            let memory_requirements = device.get_image_memory_requirements(image);
            let memory_index = find_memorytype_index(
                &memory_requirements,
                &context.memory_properties,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
            )
            .expect("Unable to find suitable memory type for image");
            let mut export_handle_info =
                vk::ExportMemoryAllocateInfo::builder().handle_types(handle_type);
            let memory = context
                .device
                .allocate_memory(
                    &vk::MemoryAllocateInfo::builder()
                        .allocation_size(memory_requirements.size)
                        .memory_type_index(memory_index)
                        .push_next(&mut export_handle_info),
                    None,
                )
                .unwrap();

            device.bind_image_memory(image, memory, 0).unwrap();

            #[cfg(target_os = "windows")]
            let external_memory =
                ash::extensions::khr::ExternalMemoryWin32::new(&context.instance, &context.device);
            #[cfg(target_os = "windows")]
            let handle = external_memory
                .get_memory_win32_handle(
                    &vk::MemoryGetWin32HandleInfoKHR::builder()
                        .handle_type(handle_type)
                        .memory(memory),
                )
                .unwrap();
            #[cfg(not(target_os = "windows"))]
            let external_memory =
                ash::extensions::khr::ExternalMemoryFd::new(&context.instance, &context.device);
            #[cfg(not(target_os = "windows"))]
            let handle = external_memory
                .get_memory_fd(
                    &vk::MemoryGetFdInfoKHR::builder()
                        .handle_type(handle_type)
                        .memory(memory),
                )
                .unwrap();
            debug!("Created handle {handle:?}");

            (image, handle)
        })
        .unzip()
}
