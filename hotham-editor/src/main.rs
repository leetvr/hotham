use ash::vk;
use lazy_vulkan::{
    find_memorytype_index, vulkan_texture::VulkanTexture, DrawCall, LazyRenderer, LazyVulkan,
    SwapchainInfo, Vertex,
};
use log::{debug, info};
use std::io::{Read, Write};
use uds_windows::{UnixListener, UnixStream};
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::ControlFlow,
    platform::run_return::EventLoopExtRunReturn,
};

/// Compile your own damn shaders! LazyVulkan is just as lazy as you are!
static FRAGMENT_SHADER: &'_ [u8] = include_bytes!("shaders/triangle.frag.spv");
static VERTEX_SHADER: &'_ [u8] = include_bytes!("shaders/triangle.vert.spv");
const SWAPCHAIN_FORMAT: vk::Format = vk::Format::R8G8B8A8_UNORM;
static UNIX_SOCKET_PATH: &'_ str = "hotham_editor.socket";

pub fn main() {
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
    let (images, image_memory_handles) =
        unsafe { create_render_images(lazy_vulkan.context(), &swapchain_info) };
    let (semaphores, semaphore_handles) =
        unsafe { create_semaphores(lazy_vulkan.context(), swapchain_info.image_count) };
    let textures = create_render_textures(lazy_vulkan.context(), &mut lazy_renderer, images);
    let (mut stream, _) = listener.accept().unwrap();
    info!("Client connected!");
    let mut buf: [u8; 1024] = [0; 1024];
    send_swapchain_info(&mut stream, &swapchain_info, &mut buf).unwrap();
    send_image_memory_handles(&mut stream, image_memory_handles, &mut buf).unwrap();
    send_semaphore_handles(&mut stream, semaphore_handles, &mut buf);

    // Off we go!
    let mut winit_initializing = true;
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

            Event::NewEvents(cause) => {
                if cause == winit::event::StartCause::Init {
                    winit_initializing = true;
                } else {
                    winit_initializing = false;
                }
            }

            Event::MainEventsCleared => {
                let framebuffer_index = lazy_vulkan.render_begin();
                send_swapchain_image_index(&mut stream, &mut buf, framebuffer_index);
                get_render_complete(&mut stream, &mut buf);
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

                let semaphore = semaphores[framebuffer_index as usize];
                lazy_vulkan.render_end(
                    framebuffer_index,
                    &[semaphore, lazy_vulkan.rendering_complete_semaphore],
                );
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
            _ => (),
        }
    });

    // I guess we better do this or else the Dreaded Validation Layers will complain
    unsafe {
        lazy_renderer.cleanup(&lazy_vulkan.context().device);
    }
}

fn send_semaphore_handles(
    stream: &mut UnixStream,
    semaphore_handles: Vec<*mut std::ffi::c_void>,
    buf: &mut [u8; 1024],
) {
    stream.read(buf).unwrap();
    let value = buf[0];
    debug!("Read {value}");

    debug!("Sending handles: {semaphore_handles:?}");
    let write = stream.write(bytes_of_slice(&semaphore_handles)).unwrap();
    debug!("Wrote {write} bytes");
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

fn get_render_complete(stream: &mut UnixStream, buf: &mut [u8]) {
    stream.read(buf).unwrap();
}

fn send_swapchain_image_index(
    stream: &mut UnixStream,
    buf: &mut [u8; 1024],
    framebuffer_index: u32,
) {
    stream.read(buf).unwrap();
    stream.write(&mut [framebuffer_index as u8]).unwrap();
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
                        array_layers: 1,
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

fn send_swapchain_info(
    stream: &mut UnixStream,
    swapchain_info: &SwapchainInfo,
    buf: &mut [u8],
) -> std::io::Result<()> {
    stream.read(buf)?;
    let value = buf[0];
    debug!("Read {value}");

    if value == 0 {
        let write = stream.write(bytes_of(swapchain_info)).unwrap();
        debug!("Write {write} bytes");
        return Ok(());
    } else {
        panic!("Invalid request!");
    }
}

fn send_image_memory_handles(
    stream: &mut UnixStream,
    handles: Vec<vk::HANDLE>,
    buf: &mut [u8],
) -> std::io::Result<()> {
    stream.read(buf)?;
    let value = buf[0];
    debug!("Read {value}");

    if value == 1 {
        debug!("Sending handles: {handles:?}");
        let write = stream.write(bytes_of_slice(&handles)).unwrap();
        debug!("Write {write} bytes");
        return Ok(());
    } else {
        panic!("Invalid request!");
    }
}

fn bytes_of_slice<T>(t: &[T]) -> &[u8] {
    unsafe {
        let ptr = t.as_ptr();
        std::slice::from_raw_parts(ptr.cast(), std::mem::size_of::<T>() * t.len())
    }
}

fn bytes_of<T>(t: &T) -> &[u8] {
    unsafe {
        let ptr = t as *const T;
        std::slice::from_raw_parts(ptr.cast(), std::mem::size_of::<T>())
    }
}
