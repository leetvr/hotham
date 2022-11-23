use crate::{
    asset_importer::{self, add_model_to_world},
    components::{GlobalTransform, Info, LocalTransform, Parent, Stage, HMD},
    contexts::{
        AudioContext, GuiContext, HapticContext, InputContext, PhysicsContext, RenderContext,
        VulkanContext, XrContext, XrContextBuilder,
    },
    workers::Workers,
    HothamError, HothamResult, VIEW_TYPE,
};
use openxr as xr;

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::sleep,
    time::{Duration, Instant},
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
    openxr_extensions: Option<xr::ExtensionSet>,
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

    /// Set the required OpenXR extensions
    pub fn openxr_extensions(&mut self, extensions: Option<xr::ExtensionSet>) -> &mut Self {
        self.openxr_extensions = extensions;
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

        // Now initialize the engine.
        let (xr_context, vulkan_context) = XrContextBuilder::new()
            .application_name(self.application_name)
            .application_version(self.application_version)
            .required_extensions(self.openxr_extensions)
            .build()
            .expect("!!FATAL ERROR - Unable to initialize OpenXR!!");
        let render_context = RenderContext::new(&vulkan_context, &xr_context)
            .expect("!!FATAL ERROR - Unable to initialize renderer!");
        let gui_context = GuiContext::new(&vulkan_context);

        // Initialize the world with our "tracking" entities, the stage and the HMD.
        let mut world = hecs::World::default();
        let (stage_entity, hmd_entity) = create_tracking_entities(&mut world);

        Engine {
            world,
            should_quit,
            resumed,
            event_data_buffer: Default::default(),
            xr_context,
            vulkan_context,
            render_context,
            audio_context: Default::default(),
            gui_context,
            haptic_context: Default::default(),
            input_context: Default::default(),
            physics_context: Default::default(),
            stage_entity,
            hmd_entity,
            performance_timers: Default::default(),
            workers: Workers::new(Default::default()),
        }
    }
}

fn create_tracking_entities(world: &mut hecs::World) -> (hecs::Entity, hecs::Entity) {
    let stage_entity = world.spawn((
        Stage {},
        LocalTransform::default(),
        GlobalTransform::default(),
    ));
    let hmd_entity = world.spawn((
        HMD {},
        Parent(stage_entity),
        LocalTransform::default(),
        GlobalTransform::default(),
    ));
    (stage_entity, hmd_entity)
}

/// The Hotham Engine
/// A wrapper around the "external world" from the perspective of the engine, eg. renderer, XR, etc.
/// **IMPORTANT**: make sure you call `update` each tick
pub struct Engine {
    should_quit: Arc<AtomicBool>,
    #[allow(dead_code)]
    resumed: bool,
    event_data_buffer: EventDataBuffer,
    /// World
    pub world: hecs::World,
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
    /// Input context
    pub input_context: InputContext,
    /// Stage entity
    pub stage_entity: hecs::Entity,
    /// HMD entity
    pub hmd_entity: hecs::Entity,
    /// Performance timers
    pub performance_timers: PerformanceTimers,
    /// Workers
    workers: Workers,
}

#[derive(Debug)]
pub struct PerformanceTimers {
    pub frame_start: Instant,
    pub timings: Vec<usize>,
    pub last_update: Instant,
}
impl PerformanceTimers {
    fn start(&mut self) {
        self.frame_start = Instant::now();
    }

    fn end(&mut self) {
        let now = Instant::now();
        let tic_time = now - self.frame_start;
        self.timings.push(tic_time.as_millis() as usize);

        if (now - self.last_update).as_secs_f32() >= 1.0 {
            let average = self.timings.iter().fold(0, |a, b| a + b) / self.timings.len();
            println!("[HOTHAM_PERF] Average tic time: {average}");
            self.last_update = now;
            self.timings.clear();
        }
    }
}

impl Default for PerformanceTimers {
    fn default() -> Self {
        Self {
            frame_start: Instant::now(),
            last_update: Instant::now(),
            timings: Default::default(),
        }
    }
}

/// The result of calling `update()` on Engine.
pub struct TickData {
    /// The previous XR state.
    pub previous_state: xr::SessionState,
    /// The current XR state.
    pub current_state: xr::SessionState,
    /// The index of the currently acquired image on the OpenXR swapchain
    pub swapchain_image_index: usize,
}

impl Engine {
    /// Create a new instance of the engine
    /// NOTE: only one instance may be running at any one time
    pub fn new() -> Self {
        EngineBuilder::new().build()
    }

    /// IMPORTANT: Call this function each tick to update the engine's running state with OpenXR and the underlying OS
    pub fn update(&mut self) -> HothamResult<TickData> {
        loop {
            #[cfg(target_os = "android")]
            process_android_events(&mut self.resumed, &self.should_quit);

            // TODO: We *STILL* don't handle being shut down correctly. Something very odd is going on.
            // https://github.com/leetvr/hotham/issues/220
            if self.should_quit.load(Ordering::Acquire) {
                // Show's over
                println!("[HOTHAM_ENGINE] Hotham is now exiting!");
                return Err(HothamError::ShuttingDown);
            }

            let (previous_state, current_state) = {
                let previous_state = self.xr_context.session_state;
                let current_state = self.xr_context.poll_xr_event(&mut self.event_data_buffer)?;
                (previous_state, current_state)
            };

            // If we're in the FOCUSSED state, process input.
            if current_state == SessionState::FOCUSED {
                self.xr_context.update_views();
                self.input_context.update(&self.xr_context);

                // Since the HMD is parented to the Stage, its LocalTransform (ie. its transform with respect to the parent)
                // is equal to its pose in stage space.
                let hmd_in_stage = self.input_context.hmd.hmd_in_stage();
                let mut transform = self
                    .world
                    .get::<&mut LocalTransform>(self.hmd_entity)
                    .unwrap();
                transform.update_from_affine(&hmd_in_stage);
            }

            // Handle any state transitions, as required.
            match (previous_state, current_state) {
                (SessionState::STOPPING, SessionState::IDLE) => {
                    // Do nothing so we can process further events.
                    continue;
                }
                (_, SessionState::IDLE) => {
                    sleep(Duration::from_millis(100)); // Sleep to avoid thrashing the CPU
                    continue;
                }
                (SessionState::IDLE, SessionState::READY) => {
                    self.xr_context.session.begin(VIEW_TYPE)?;
                }
                (_, SessionState::EXITING | SessionState::LOSS_PENDING) => {
                    // Show's over
                    println!("[HOTHAM_ENGINE] Hotham is now exiting!");
                    return Err(HothamError::ShuttingDown);
                }
                (_, SessionState::STOPPING) => {
                    self.xr_context.end_session()?;
                    continue;
                }
                _ => {}
            }

            // Check to see if there are any messages from our workers:
            self.check_for_worker_messages();

            let vulkan_context = &self.vulkan_context;
            let render_context = &mut self.render_context;

            // In any other state, begin the frame loop.
            match self.xr_context.begin_frame() {
                Err(HothamError::NotRendering) => continue,
                Ok(swapchain_image_index) => {
                    self.performance_timers.start();
                    render_context.begin_frame(vulkan_context);
                    return Ok(TickData {
                        previous_state,
                        current_state,
                        swapchain_image_index,
                    });
                }
                err => panic!("Error beginning frame: {:?}", err),
            };
        }
    }

    /// Call this after update
    pub fn finish(&mut self) -> xr::Result<()> {
        self.performance_timers.end();
        let vulkan_context = &self.vulkan_context;
        let render_context = &mut self.render_context;

        if self.xr_context.frame_state.should_render {
            render_context.end_frame(vulkan_context);
        }
        self.xr_context.end_frame()
    }

    /// Watch some assets, just for fun.
    pub fn watch_assets(&mut self, asset_list: Vec<String>) {
        self.workers = Workers::new(asset_list);
    }

    fn check_for_worker_messages(&mut self) -> () {
        for message in self.workers.receiver.try_iter() {
            match message {
                crate::workers::WorkerMessage::AssetUpdated(asset_updated) => {
                    let tick = Instant::now();
                    let vulkan_context = &self.vulkan_context;
                    let render_context = &mut self.render_context;
                    let world = &mut self.world;

                    // First, load the asset in
                    let asset = asset_updated.asset_data;
                    let mut scene =
                        asset_importer::load_scene_from_glb(&asset, vulkan_context, render_context)
                            .unwrap();

                    let models = &scene.models;

                    // First, then remove the existing assets:
                    let mut command_buffer = hecs::CommandBuffer::default();
                    for (e, info) in world.query::<&Info>().iter() {
                        if !models.contains_key(&info.name) {
                            continue;
                        }

                        command_buffer.despawn(e);
                        despawn_children(world, e, &mut command_buffer);
                    }
                    command_buffer.run_on(world);

                    // Add the models back
                    for name in models.keys() {
                        add_model_to_world(name, &models, world, None);
                    }

                    // Clear out the lights
                    for (i, light) in scene.lights.drain(..).enumerate() {
                        render_context.scene_data.lights[i] = light;
                    }

                    println!(
                        "[HOTHAM_ASSET_HOT_RELOAD] Asset reload took {:.2} seconds",
                        Instant::now().duration_since(tick).as_secs_f32()
                    );
                }
                crate::workers::WorkerMessage::Error(e) => {
                    panic!("[HOTHAM_ENGINE] Worker enountered error: {:?}", e);
                }
            }
        }
    }
}

fn despawn_children(
    world: &hecs::World,
    parent: hecs::Entity,
    command_buffer: &mut hecs::CommandBuffer,
) {
    for (child, p) in world.query::<&Parent>().iter() {
        if p.0 == parent {
            command_buffer.despawn(child);
            despawn_children(world, child, command_buffer);
        }
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "android")]
pub fn process_android_events(resumed: &mut bool, should_quit: &Arc<AtomicBool>) {
    while let Some(event) = poll_android_events(*resumed) {
        println!("[HOTHAM_ANDROID] Received event {:?}", event);
        match event {
            ndk_glue::Event::Resume => *resumed = true,
            ndk_glue::Event::Destroy => {
                println!("[HOTHAM_ANDROID] !! DESTROY CALLED! DESTROY EVERYTHING! DESTROY!!!!");
                should_quit.store(true, Ordering::Release);
                return;
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
