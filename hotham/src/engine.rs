use crate::{resources::XrContext, HothamResult, VIEW_TYPE};
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
}

impl Drop for Engine {
    fn drop(&mut self) {
        #[cfg(target_os = "android")]
        ndk_glue::native_activity().finish();
    }
}

impl Engine {
    pub fn new() -> Self {
        Self {
            should_quit: Arc::new(AtomicBool::from(false)),
            resumed: true,
            event_data_buffer: Default::default(),
        }
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit.load(Ordering::Relaxed)
    }

    pub fn update(&mut self, xr_context: &mut XrContext) -> HothamResult<()> {
        #[cfg(not(target_os = "android"))]
        {
            let should_quit = self.should_quit.clone();
            ctrlc::set_handler(move || should_quit.store(true, Ordering::Relaxed))
                .map_err(anyhow::Error::new)?;
        }

        #[cfg(target_os = "android")]
        if self.process_android_events() {
            return Ok(());
        };

        let (previous_state, current_state) = {
            let previous_state = xr_context.session_state.clone();
            let current_state = xr_context.poll_xr_event(&mut self.event_data_buffer)?;
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
                xr_context.session.begin(VIEW_TYPE)?;
            }
            (_, SessionState::STOPPING) => {
                xr_context.end_session()?;
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

        Ok(())
    }

    #[cfg(target_os = "android")]
    pub fn process_android_events(&mut self) -> bool {
        loop {
            if let Some(event) = self.poll_android_events() {
                println!("[HOTHAM_ANDROID] Received event {:?}", event);
                match event {
                    ndk_glue::Event::Resume => self.resumed = true,
                    ndk_glue::Event::Destroy | ndk_glue::Event::WindowDestroyed => {
                        self.should_quit.store(true, Ordering::Relaxed);
                        return true;
                    }
                    ndk_glue::Event::Pause => self.resumed = false,
                    _ => {}
                }
            } else {
                break;
            }
        }

        false
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
