mod camera;
mod gui;
mod input_context;

use anyhow::{bail, Result};
use ash::vk;

use glam::Vec2;
use hotham_editor_protocol::{responses, scene::EditorUpdates, EditorServer, RequestType};
use lazy_vulkan::{
    find_memorytype_index, vulkan_context::VulkanContext, vulkan_texture::VulkanTexture,
    LazyRenderer, LazyVulkan, SwapchainInfo, Vertex,
};
use log::{debug, info, trace};
use yakui_winit::YakuiWinit;

use std::time::Instant;
use uds_windows::{UnixListener, UnixStream};
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::ControlFlow,
    platform::run_return::EventLoopExtRunReturn,
};

use crate::{
    camera::Camera,
    gui::{gui, GuiState},
};

static FRAGMENT_SHADER: &'_ [u8] = include_bytes!("shaders/triangle.frag.spv");
static VERTEX_SHADER: &'_ [u8] = include_bytes!("shaders/triangle.vert.spv");
const SWAPCHAIN_FORMAT: vk::Format = vk::Format::R8G8B8A8_SRGB; // OpenXR really, really wants us to use sRGB swapchains
static UNIX_SOCKET_PATH: &'_ str = "hotham_editor.socket";

pub fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let vertices = [
        Vertex::new([1.0, 1.0, 0.0, 1.0], [1.0, 1.0, 1.0, 0.0], [1.0, 1.0]), // bottom right
        Vertex::new([-1.0, 1.0, 0.0, 1.0], [1.0, 1.0, 1.0, 0.0], [0.0, 1.0]), // bottom left
        Vertex::new([1.0, -1.0, 0.0, 1.0], [1.0, 1.0, 1.0, 0.0], [1.0, 0.0]), // top right
        Vertex::new([-1.0, -1.0, 0.0, 1.0], [1.0, 1.0, 1.0, 0.0], [0.0, 0.0]), // top left
    ];

    // Your own index type?! What are you going to use, `u16`?
    let indices = [0, 1, 2, 2, 1, 3];

    let window_size = vk::Extent2D {
        width: 800,
        height: 500,
    };

    // Let's do something totally normal and wait for a TCP connection
    if std::fs::remove_file(UNIX_SOCKET_PATH).is_ok() {
        debug!("Removed pre-existing unix socket at {UNIX_SOCKET_PATH}");
    }

    let listener = UnixListener::bind(UNIX_SOCKET_PATH).unwrap();
    info!("Listening on {UNIX_SOCKET_PATH}: waiting for game client..");
    let (stream, _) = listener.accept().unwrap();
    let mut game_server = EditorServer::new(stream);

    info!("Game connected! Waiting for OpenXR client..",);
    let (stream, _) = listener.accept().unwrap();
    let mut openxr_server = EditorServer::new(stream);
    info!("OpenXR client connected! Opening window and doing OpenXR setup");

    // Alright, let's build some stuff
    let (mut lazy_vulkan, mut lazy_renderer, mut event_loop) = LazyVulkan::builder()
        .initial_vertices(&vertices)
        .initial_indices(&indices)
        .fragment_shader(FRAGMENT_SHADER)
        .vertex_shader(VERTEX_SHADER)
        .with_present(true)
        .window_size(window_size)
        .build();
    let swapchain_info = SwapchainInfo {
        image_count: lazy_vulkan.surface.desired_image_count,
        resolution: vk::Extent2D {
            width: 500,
            height: 500,
        },
        format: SWAPCHAIN_FORMAT,
    };
    let xr_swapchain = do_openxr_setup(&mut openxr_server, lazy_vulkan.context(), &swapchain_info)?;
    info!("..done!");
    let mut yak_images = create_render_textures(
        lazy_vulkan.context(),
        &mut lazy_renderer,
        xr_swapchain.images,
    );

    let window = &lazy_vulkan.window;

    let mut last_frame_time = Instant::now();
    let mut keyboard_events = Vec::new();
    let mut mouse_events = Vec::new();
    let mut camera = Camera::default();
    let mut yak = yakui::Yakui::new();
    let mut yakui_window = YakuiWinit::new(window);

    let (mut yakui_vulkan, yak_images) = {
        let context = lazy_vulkan.context();
        let yakui_vulkan_context = yakui_vulkan::VulkanContext::new(
            &context.device,
            context.queue,
            context.draw_command_buffer,
            context.command_pool,
            context.memory_properties,
        );
        let render_surface = yakui_vulkan::RenderSurface {
            resolution: window_size,
            format: lazy_vulkan.surface.surface_format.format,
            image_views: lazy_renderer.render_surface.image_views.clone(),
        };
        let mut yakui_vulkan =
            yakui_vulkan::YakuiVulkan::new(&yakui_vulkan_context, render_surface);
        let yak_images = yak_images
            .drain(..)
            .map(|t| {
                let yak_texture = lazy_to_yak(&yakui_vulkan_context, yakui_vulkan.descriptors(), t);
                yakui_vulkan.add_user_texture(yak_texture)
            })
            .collect::<Vec<_>>();

        (yakui_vulkan, yak_images)
    };

    // Off we go!
    let mut winit_initializing = true;
    let mut focused = false;
    let mut right_mouse_clicked = false;
    let mut updates = EditorUpdates {
        entity_updates: vec![],
    };

    event_loop.run_return(|event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        yakui_window.handle_event(&mut yak, &event);
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

                check_request(&mut openxr_server, RequestType::LocateView).unwrap();
                openxr_server.send_response(&camera.as_pose()).unwrap();

                check_request(&mut openxr_server, RequestType::WaitFrame).unwrap();
                openxr_server.send_response(&0).unwrap();

                check_request(&mut openxr_server, RequestType::AcquireSwapchainImage).unwrap();
                openxr_server.send_response(&framebuffer_index).unwrap();

                let scene: hotham_editor_protocol::scene::Scene = game_server.get_json().unwrap();
                let mut gui_state = GuiState { texture_id: yak_images[framebuffer_index as usize], scene, updates: vec![] };
                game_server
                    .send_json(&updates)
                    .unwrap();

                updates.entity_updates.clear();

                // game has finished rendering its frame here

                check_request(&mut openxr_server, RequestType::LocateView).unwrap();
                openxr_server.send_response(&camera.as_pose()).unwrap();

                check_request(&mut openxr_server, RequestType::EndFrame).unwrap();
                openxr_server.send_response(&0).unwrap();


                yak.start();
                gui(&mut gui_state);
                yak.finish();

                updates.entity_updates = gui_state.updates;


                let context = lazy_vulkan.context();
                let yakui_vulkan_context = yakui_vulkan::VulkanContext::new(
                    &context.device,
                    context.queue,
                    context.draw_command_buffer,
                    context.command_pool,
                    context.memory_properties,
                );

                yakui_vulkan.paint(&mut yak, &yakui_vulkan_context, framebuffer_index);

                let semaphore = xr_swapchain.semaphores[framebuffer_index as usize];
                lazy_vulkan.render_end(
                    framebuffer_index,
                    &[semaphore, lazy_vulkan.rendering_complete_semaphore],
                );
                last_frame_time = Instant::now();
                right_mouse_clicked = false;
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                if !winit_initializing {
                    let new_render_surface = lazy_vulkan.resized(size.width, size.height);
                    let render_surface = yakui_vulkan::RenderSurface {
                        resolution: new_render_surface.resolution,
                        format: new_render_surface.format,
                        image_views: new_render_surface.image_views,
                    };
                    yakui_vulkan.update_surface(render_surface, &lazy_vulkan.context().device);
                    yak.set_surface_size([size.width as f32, size.height as f32].into());
                }
            }
            Event::WindowEvent {
                event:
                    WindowEvent::ScaleFactorChanged {
                        scale_factor,
                        new_inner_size,
                    },
                ..
            } => {
                debug!("Scale factor changed! Scale factor: {scale_factor}, new inner size: {new_inner_size:?}");
            }
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput { input, .. },
                ..
            } => {
                if focused && input.virtual_keycode.is_some() {
                    keyboard_events.push(input);
                }
            }
            Event::WindowEvent {
                event: WindowEvent::CursorMoved { .. },
                ..
            } => {
                focused = true;
            }
            Event::WindowEvent {
                event: WindowEvent::CursorLeft { .. },
                ..
            } => {
                focused = false;
            }
            Event::DeviceEvent { event, .. } => match event {
                winit::event::DeviceEvent::MouseMotion { delta } => {
                    if focused {
                        mouse_events.push(MouseInput::MouseMoved(
                            [delta.0 as f32, delta.1 as f32].into(),
                        ))
                    }
                }
                winit::event::DeviceEvent::Button {
                    button: 3,
                    state: ElementState::Pressed,
                } => {
                    debug!("Right mouse clicked");
                    right_mouse_clicked = true;
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

fn lazy_to_yak(
    yakui_vulkan_context: &yakui_vulkan::VulkanContext,
    descriptors: &mut yakui_vulkan::Descriptors,
    t: VulkanTexture,
) -> yakui_vulkan::VulkanTexture {
    yakui_vulkan::VulkanTexture::from_image(
        yakui_vulkan_context,
        descriptors,
        t.image,
        t.memory,
        t.view,
    )
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
        unsafe { create_render_images(vulkan_context, swapchain_info) };
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
    trace!("Received request from cilent {expected_request_type:?}");
    Ok(())
}

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

unsafe fn create_render_images(
    context: &lazy_vulkan::vulkan_context::VulkanContext,
    swapchain_info: &SwapchainInfo,
) -> (Vec<vk::Image>, Vec<vk::HANDLE>) {
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

            let external_memory =
                ash::extensions::khr::ExternalMemoryWin32::new(&context.instance, &context.device);
            let handle = external_memory
                .get_memory_win32_handle(
                    &vk::MemoryGetWin32HandleInfoKHR::builder()
                        .handle_type(handle_type)
                        .memory(memory),
                )
                .unwrap();
            debug!("Created handle {handle:?}");

            (image, handle)
        })
        .unzip()
}
