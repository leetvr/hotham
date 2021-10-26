use crate::{resources::XrContext, HothamResult};
use legion::{Resources, Schedule, World};
use openxr as xr;

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::sleep,
    time::Duration,
};

use xr::SessionState;

#[cfg(target_os = "android")]
pub const ANDROID_LOOPER_ID_MAIN: u32 = 0;
#[cfg(target_os = "android")]
pub const ANDROID_LOOPER_ID_INPUT: u32 = 1;
#[cfg(target_os = "android")]
pub const ANDROID_LOOPER_NONBLOCKING_TIMEOUT: Duration = Duration::from_millis(0);
#[cfg(target_os = "android")]
pub const ANDROID_LOOPER_BLOCKING_TIMEOUT: Duration = Duration::from_millis(i32::MAX as _);

pub struct App {
    should_quit: Arc<AtomicBool>,
    world: World,
    resources: Resources,
    schedule: Schedule,
    #[allow(dead_code)]
    resumed: bool,
}

impl App {
    pub fn new(world: World, resources: Resources, schedule: Schedule) -> HothamResult<Self> {
        Ok(Self {
            should_quit: Arc::new(AtomicBool::from(false)),
            resumed: true,
            world,
            resources,
            schedule,
        })
    }

    pub fn run(&mut self) -> HothamResult<()> {
        #[cfg(not(target_os = "android"))]
        {
            let should_quit = self.should_quit.clone();
            ctrlc::set_handler(move || should_quit.store(true, Ordering::Relaxed))
                .map_err(anyhow::Error::new)?;
        }

        let mut event_buffer = Default::default();

        while !self.should_quit.load(Ordering::Relaxed) {
            #[cfg(target_os = "android")]
            self.process_android_events();

            let mut xr_context = self.resources.get_mut::<XrContext>().unwrap();
            let current_state = xr_context.poll_xr_event(&mut event_buffer)?;

            if current_state == SessionState::IDLE {
                sleep(Duration::from_secs(1));
                continue;
            }

            if current_state == SessionState::EXITING {
                break;
            }

            drop(xr_context);

            self.schedule.execute(&mut self.world, &mut self.resources);
        }

        Ok(())
    }

    #[cfg(target_os = "android")]
    pub fn process_android_events(&mut self) {
        loop {
            if let Some(event) = self.poll_android_events() {
                println!("[HOTHAM_ANDROID] Received event {:?}", event);
                match event {
                    ndk_glue::Event::Resume => self.resumed = true,
                    ndk_glue::Event::Destroy => self.should_quit.store(true, Ordering::Relaxed),
                    ndk_glue::Event::Pause => self.resumed = false,
                    _ => {}
                }
            }
            break;
        }
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
