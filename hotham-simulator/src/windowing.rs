use std::sync::{mpsc::Sender, Arc};

use ash::vk;

pub fn create_window_thread(
    _entry: ash::Entry,
    _instance: ash::Instance,
    _swapchain_ext: ash::khr::swapchain::Device,
    _close_window: Arc<std::sync::atomic::AtomicBool>,
    _swapchain_tx: Sender<(vk::SurfaceKHR, vk::SwapchainKHR)>,
    _mouse_event_tx: Sender<(f64, f64)>,
    _keyboard_event_tx: Sender<winit::event::KeyEvent>,
) -> std::thread::JoinHandle<()> {
    // No longer possible. See: https://github.com/leetvr/hotham/pull/450
    unimplemented!();
}

pub fn get_window_extensions() -> Vec<&'static std::ffi::CStr> {
    // No longer possible. See: https://github.com/leetvr/hotham/pull/450
    unimplemented!();
}
