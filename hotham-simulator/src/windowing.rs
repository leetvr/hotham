use std::{
    cell::RefCell,
    ffi::CStr,
    sync::{atomic::Ordering, mpsc::Sender, Arc},
    thread,
};

use ash::vk::{self, SurfaceKHR, SwapchainKHR};
use winit::{
    dpi::PhysicalSize,
    event::{DeviceEvent, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::WindowAttributes,
};

#[cfg(target_os = "linux")]
use winit::platform::x11::EventLoopBuilderExtX11;

#[cfg(target_os = "windows")]
use winit::platform::windows::EventLoopBuilderExtWindows;

use crate::simulator::{SWAPCHAIN_COLOR_FORMAT, VIEWPORT_HEIGHT, VIEWPORT_WIDTH};

thread_local! {
    static EVENT_LOOP: RefCell<Option<EventLoop<()>>> = RefCell::new(None)
}

pub fn create_window_thread(
    entry: ash::Entry,
    instance: ash::Instance,
    swapchain_ext: ash::khr::swapchain::Device,
    close_window: Arc<std::sync::atomic::AtomicBool>,
    swapchain_tx: Sender<(SurfaceKHR, SwapchainKHR)>,
    mouse_event_tx: Sender<(f64, f64)>,
    keyboard_event_tx: Sender<winit::event::KeyEvent>,
) -> thread::JoinHandle<()> {
    use winit::event::{ElementState, MouseButton};

    // Extract the eventloop from its RefCell
    let mut event_loop = None;
    EVENT_LOOP.with(|slot| {
        event_loop = slot.replace(None);
    });
    let event_loop = event_loop.unwrap();
    let proxy = event_loop.create_proxy();

    let window_thread_handle = thread::spawn(move || {
        println!("[HOTHAM_SIMULATOR] Creating window..");

        // We cannot easily switch to winit's new AppHandler model so instead we just do it The Old Way.

        #[allow(deprecated)]
        let window = proxy
            .create_window(
                WindowAttributes::default()
                    .with_inner_size(PhysicalSize::new(VIEWPORT_WIDTH, VIEWPORT_HEIGHT))
                    .with_title("Hotham Simulator"),
            )
            .unwrap();
        println!("[HOTHAM_SIMULATOR] ..done.");

        let display_handle = event_loop.display_handle().unwrap();
        let window_handle = window.window_handle().unwrap();

        println!("[HOTHAM_SIMULATOR] Creating surface..");
        let surface = unsafe {
            ash_window::create_surface(
                &entry,
                &instance,
                display_handle.as_raw(),
                window_handle.as_raw(),
                None,
            )
            .unwrap()
        };
        println!("[HOTHAM_SIMULATOR] ..done");

        let extent = vk::Extent2D {
            height: VIEWPORT_HEIGHT,
            width: VIEWPORT_WIDTH,
        };
        // Create a swapchain
        println!("[HOTHAM_SIMULATOR] About to create swapchain..");

        let swapchain = unsafe {
            swapchain_ext
                .create_swapchain(
                    &vk::SwapchainCreateInfoKHR::default()
                        .min_image_count(3)
                        .surface(surface)
                        .image_format(SWAPCHAIN_COLOR_FORMAT)
                        .image_color_space(vk::ColorSpaceKHR::SRGB_NONLINEAR)
                        .image_array_layers(1)
                        .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                        .image_extent(extent)
                        .queue_family_indices(&[])
                        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                        .present_mode(vk::PresentModeKHR::IMMEDIATE)
                        .clipped(true)
                        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT),
                    None,
                )
                .unwrap()
        };
        println!("[HOTHAM_SIMULATOR] Created swapchain: {swapchain:?}. Sending..");

        // Send the swapchain back to the caller
        swapchain_tx.send((surface, swapchain)).unwrap();

        let cl2 = close_window.clone();

        let mut mouse_pressed = false;

        event_loop.set_control_flow(ControlFlow::Poll);

        #[allow(deprecated)]
        event_loop
            .run(move |event, active_event_loop| {
                if close_window.load(Ordering::Relaxed) {
                    println!("[HOTHAM_SIMULATOR] Closed called!");
                    active_event_loop.exit();
                }

                match event {
                    Event::WindowEvent { event, .. } => match event {
                        WindowEvent::CloseRequested => {
                            active_event_loop.exit();
                        }
                        WindowEvent::KeyboardInput { event, .. } => {
                            keyboard_event_tx.send(event).unwrap()
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
                    Event::DeviceEvent { event, .. } => {
                        if mouse_pressed {
                            if let DeviceEvent::MouseMotion { delta } = event {
                                mouse_event_tx.send(delta).unwrap();
                            }
                        }
                    }
                    _ => (),
                }
            })
            .unwrap();

        cl2.store(true, Ordering::Relaxed);
    });
    window_thread_handle
}

pub fn get_window_extensions() -> Vec<&'static CStr> {
    println!("[HOTHAM_SIMULATOR] Getting window extensions..");
    let mut extensions = Vec::new();

    // Sigh. EventLoop can't be created multiple times, so we stash it in a thread local
    EVENT_LOOP.with(|slot| {
        if slot.borrow().is_none() {
            let event_loop = EventLoop::builder().with_any_thread(true).build().unwrap();
            *slot.borrow_mut() = Some(event_loop);
        }

        let event_loop = slot.borrow();
        let event_loop = event_loop.as_ref().unwrap();

        extensions = ash_window::enumerate_required_extensions(
            event_loop.display_handle().unwrap().as_raw(),
        )
        .unwrap()
        .iter()
        .map(|p| unsafe { CStr::from_ptr(*p) })
        .collect();
    });

    extensions
}
