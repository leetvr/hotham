use crate::{
    resources::{
        AudioContext, GuiContext, HapticContext, PhysicsContext, RenderContext, VulkanContext,
        XrContext, XrContextBuilder,
    },
    HothamError, HothamResult, VIEW_TYPE,
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
pub static ANDROID_LOOPER_ID_MAIN: u32 = 0;
#[cfg(target_os = "android")]
pub static ANDROID_LOOPER_NONBLOCKING_TIMEOUT: Duration = Duration::from_millis(0);
#[cfg(target_os = "android")]
pub static ANDROID_LOOPER_BLOCKING_TIMEOUT: Duration = Duration::from_millis(i32::MAX as _);

/// Builder for `Engine`.
#[derive(Default)]
pub struct EngineBuilder<'a> {
    application_name: Option<&'a str>,
    application_version: Option<u32>,
}

impl<'a> EngineBuilder<'a> {
    /// Create an `EngineBuilder`
    pub fn new() -> Self {
        Default::default()
    }

    /// Set the OpenXR application name
    pub fn application_name(&mut self, name: Option<&'a str>) -> &mut Self {
        self.application_name = name;
        self
    }

    /// Set the OpenXR application version
    pub fn application_version(&mut self, version: Option<u32>) -> &mut Self {
        self.application_version = version;
        self
    }

    /// Build the `Engine`
    pub fn build(self) -> Engine {
        #[allow(unused_mut)] // Only Android mutates this.
        let mut resumed = false;
        let should_quit = Arc::new(AtomicBool::from(false));

        // Before we do ANYTHING - we should process android events
        #[cfg(target_os = "android")]
        process_android_events(&mut resumed, &should_quit);

        // On desktop, register a Ctrl-C handler.
        #[cfg(not(target_os = "android"))]
        {
            let should_quit = should_quit.clone();
            ctrlc::set_handler(move || should_quit.store(true, Ordering::Relaxed)).unwrap();
        }

        // Now initialise the engine.
        let (xr_context, vulkan_context) = XrContextBuilder::new()
            .application_name(self.application_name)
            .application_version(self.application_version)
            .build()
            .expect("!!FATAL ERROR - Unable to initialise OpenXR!!");
        let render_context = RenderContext::new(&vulkan_context, &xr_context)
            .expect("!!FATAL ERROR - Unable to initialise renderer!");
        let gui_context = GuiContext::new(&vulkan_context);

        let mut engine = Engine {
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
}

/// The Hotham Engine
/// A wrapper around the "external world" from the perspective of the engine, eg. renderer, XR, etc.
/// **IMPORTANT**: make sure you call `update` each tick
pub struct Engine {
    should_quit: Arc<AtomicBool>,
    #[allow(dead_code)]
    resumed: bool,
    event_data_buffer: EventDataBuffer,

    /// OpenXR context
    pub xr_context: XrContext,
    /// Vulkan context
    pub vulkan_context: VulkanContext,
    /// Renderer context
    pub render_context: RenderContext,
    /// Physics context
    pub physics_context: PhysicsContext,
    /// Audio context
    pub audio_context: AudioContext,
    /// GUI context
    pub gui_context: GuiContext,
    /// Haptics context
    pub haptic_context: HapticContext,
}

impl Engine {
    /// Create a new instance of the engine
    /// NOTE: only one instance may be running at any one time
    pub fn new() -> Self {
        EngineBuilder::new().build()
    }

    /// IMPORTANT: Call this function each tick to update the engine's running state with the underlying OS
    pub fn update(&mut self) -> HothamResult<(xr::SessionState, xr::SessionState)> {
        #[cfg(target_os = "android")]
        process_android_events(&mut self.resumed, &self.should_quit);

        let (previous_state, current_state) = {
            let previous_state = self.xr_context.session_state;
            let current_state = self.xr_context.poll_xr_event(&mut self.event_data_buffer)?;
            (previous_state, current_state)
        };

        match (previous_state, current_state) {
            (SessionState::STOPPING, SessionState::IDLE) => {
                // Do nothing so we can process further events.
            }
            (_, SessionState::IDLE) => {
                sleep(Duration::from_millis(100)); // Sleep to avoid thrasing the CPU
            }
            (SessionState::IDLE, SessionState::READY) => {
                self.xr_context.session.begin(VIEW_TYPE)?;
            }
            (_, SessionState::EXITING) => {
                // Show's over
                println!("[HOTHAM_ENGINE] State is now exiting!");
                return Err(HothamError::ShuttingDown);
            }
            (_, SessionState::STOPPING) => {
                self.xr_context.end_session()?;
            }
            _ => {}
        }

        if self.should_quit.load(Ordering::Relaxed) {
            return Err(HothamError::ShuttingDown);
        }

        Ok((previous_state, current_state))
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "android")]
pub fn process_android_events(resumed: &mut bool, should_quit: &Arc<AtomicBool>) -> bool {
    while let Some(event) = poll_android_events(*resumed) {
        println!("[HOTHAM_ANDROID] Received event {:?}", event);
        match event {
            ndk_glue::Event::Resume => *resumed = true,
            ndk_glue::Event::Destroy => {
                should_quit.store(true, Ordering::Relaxed);
                return true;
            }
            ndk_glue::Event::Pause => *resumed = false,
            _ => {}
        }
    }

    if let Some(ref input_queue) = *ndk_glue::input_queue() {
        while let Some(event) = input_queue.get_event() {
            if let Some(event) = input_queue.pre_dispatch(event) {
                input_queue.finish_event(event, false);
            }
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
            } else {
                None
            }
        }
        _ => None,
    }
}
