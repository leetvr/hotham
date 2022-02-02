use crate::{
    resources::{
        AudioContext, GuiContext, HapticContext, PhysicsContext, RenderContext, VulkanContext,
        XrContext,
    },
    HothamResult, VIEW_TYPE,
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

use xr::{EventDataBuffer, SessionState};

#[cfg(target_os = "android")]
pub const ANDROID_LOOPER_ID_MAIN: u32 = 0;
#[cfg(target_os = "android")]
pub const ANDROID_LOOPER_ID_INPUT: u32 = 1;
#[cfg(target_os = "android")]
pub const ANDROID_LOOPER_NONBLOCKING_TIMEOUT: Duration = Duration::from_millis(0);
#[cfg(target_os = "android")]
pub const ANDROID_LOOPER_BLOCKING_TIMEOUT: Duration = Duration::from_millis(i32::MAX as _);

pub struct Engine {
    should_quit: Arc<AtomicBool>,
    #[allow(dead_code)]
    resumed: bool,
    event_data_buffer: EventDataBuffer,
    pub xr_context: XrContext,
    pub vulkan_context: VulkanContext,
    pub render_context: RenderContext,
    pub physics_context: PhysicsContext,
    pub audio_context: AudioContext,
    pub gui_context: GuiContext,
    pub haptic_context: HapticContext,
}

impl Drop for Engine {
    fn drop(&mut self) {
        #[cfg(target_os = "android")]
        ndk_glue::native_activity().finish();
    }
}

impl Engine {
    pub fn new() -> Self {
        let mut resumed = false;
        let should_quit = Arc::new(AtomicBool::from(false));

        // Before we do ANYTHING - we must process android events
        #[cfg(target_os = "android")]
        process_android_events(&mut resumed, &should_quit);

        // On desktop, register a Ctrl-C handler.
        #[cfg(not(target_os = "android"))]
        {
            let should_quit = should_quit.clone();
            ctrlc::set_handler(move || should_quit.store(true, Ordering::Relaxed)).unwrap();
        }

        // Now initialise the engine.
        let (xr_context, vulkan_context) =
            XrContext::new().expect("!!FATAL ERROR - Unable to initialise OpenXR!!");
        let render_context = RenderContext::new(&vulkan_context, &xr_context)
            .expect("!!FATAL ERROR - Unable to initialise renderer!");
        let gui_context = GuiContext::new(&vulkan_context);

        let mut engine = Self {
            should_quit,
            resumed,
            event_data_buffer: Default::default(),
            xr_context,
            vulkan_context,
            render_context,
            physics_context: Default::default(),
            audio_context: Default::default(),
            gui_context,
            haptic_context: Default::default(),
        };

        engine.update().unwrap();
        engine
    }

    pub fn update(&mut self) -> HothamResult<bool> {
        #[cfg(target_os = "android")]
        if process_android_events(&mut self.resumed, &self.should_quit) {
            return Ok(false);
        };

        let (previous_state, current_state) = {
            let previous_state = self.xr_context.session_state.clone();
            let current_state = self.xr_context.poll_xr_event(&mut self.event_data_buffer)?;
            (previous_state, current_state)
        };

        match (previous_state, current_state) {
            (SessionState::EXITING, SessionState::IDLE) => {
                // return quickly so we can process the Android lifecycle
            }
            (_, SessionState::IDLE) => {
                sleep(Duration::from_millis(100)); // Sleep to avoid thrasing the CPU
            }
            (SessionState::IDLE, SessionState::READY) => {
                self.xr_context.session.begin(VIEW_TYPE)?;
            }
            (_, SessionState::STOPPING) => {
                self.xr_context.end_session()?;
            }
            (
                _,
                SessionState::EXITING
                | SessionState::LOSS_PENDING
                | SessionState::SYNCHRONIZED
                | SessionState::VISIBLE
                | SessionState::FOCUSED,
            ) => {}
            _ => println!(
                "[HOTHAM_MAIN] - Unhandled - previous: {:?}, current: {:?}",
                previous_state, current_state
            ),
        }

        Ok(!self.should_quit.load(Ordering::Acquire))
    }
}

#[cfg(target_os = "android")]
pub fn process_android_events(resumed: &mut bool, should_quit: &Arc<AtomicBool>) -> bool {
    while let Some(event) = poll_android_events(*resumed) {
        println!("[HOTHAM_ANDROID] Received event {:?}", event);
        match event {
            ndk_glue::Event::Resume => *resumed = true,
            ndk_glue::Event::Destroy | ndk_glue::Event::WindowDestroyed => {
                should_quit.store(true, Ordering::Relaxed);
                return true;
            }
            ndk_glue::Event::Pause => *resumed = false,
            _ => {}
        }
    }

    false
}

#[cfg(target_os = "android")]
pub fn poll_android_events(resumed: bool) -> Option<ndk_glue::Event> {
    use ndk::looper::{Poll, ThreadLooper};

    let looper = ThreadLooper::for_thread().unwrap();
    let timeout = if resumed {
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
