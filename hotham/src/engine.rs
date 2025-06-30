use crate::{
    asset_importer::{self, add_model_to_world},
    components::{GlobalTransform, Info, LocalTransform, Parent, Stage, HMD},
    contexts::{
        render_context::create_pipeline, AudioContext, GuiContext, HapticContext, InputContext,
        PhysicsContext, RenderContext, VulkanContext, XrContext, XrContextBuilder,
    },
    util::{u8_to_u32, PerformanceTimer},
    workers::Workers,
    HothamError, HothamResult, VIEW_TYPE,
};
use hecs::Entity;
use hotham_asset_client::AssetUpdatedMessage;
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
            performance_timer: PerformanceTimer::new("Application Tick"),
            graveyard: Default::default(),
            recently_updated_assets: Default::default(),
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
    pub performance_timer: PerformanceTimer,
    /// Entity graveyard
    pub graveyard: Vec<Entity>,
    /// Files that were hot reloaded this frame
    recently_updated_assets: Vec<AssetUpdatedMessage>,
    /// Workers
    workers: Workers,
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
                    render_context.begin_frame(vulkan_context);
                    self.performance_timer.start();
                    return Ok(TickData {
                        previous_state,
                        current_state,
                        swapchain_image_index,
                    });
                }
                err => panic!("Error beginning frame: {err:?}"),
            };
        }
    }

    /// Call this after update
    pub fn finish(&mut self) -> xr::Result<()> {
        self.empty_graveyard();
        self.performance_timer.end();
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

    /// Get a list of assets updated this frame.
    pub fn get_updated_assets(&self) -> &Vec<AssetUpdatedMessage> {
        &self.recently_updated_assets
    }

    fn check_for_worker_messages(&mut self) {
        self.recently_updated_assets.clear();
        for message in self.workers.receiver.try_iter() {
            match message {
                crate::workers::WorkerMessage::AssetUpdated(asset_updated) => {
                    let tick = Instant::now();
                    let vulkan_context = &self.vulkan_context;
                    let render_context = &mut self.render_context;
                    let world = &mut self.world;

                    let file_type = asset_updated.asset_id.split('.').next_back().unwrap();
                    match (asset_updated.asset_id.as_str(), file_type) {
                        (_, "glb") => update_models(
                            vulkan_context,
                            render_context,
                            world,
                            asset_updated.asset_data.clone(),
                        ),
                        ("hotham/src/shaders/pbr.frag.spv", _)
                        | ("hotham/src/shaders/pbr.vert.spv", _) => update_shader(
                            vulkan_context,
                            render_context,
                            &asset_updated.asset_id,
                            asset_updated.asset_data.clone(),
                        ),
                        _ => {}
                    }
                    self.recently_updated_assets.push(asset_updated);
                    println!(
                        "[HOTHAM_ASSET_HOT_RELOAD] Asset reload took {:.2} seconds",
                        Instant::now().duration_since(tick).as_secs_f32()
                    );
                }
                crate::workers::WorkerMessage::Error(e) => {
                    panic!("[HOTHAM_ENGINE] Worker encountered error: {e:?}");
                }
            }
        }
    }

    fn empty_graveyard(&mut self) {
        let world = &mut self.world;
        for entity_to_despawn in self.graveyard.drain(..) {
            let _ = world.despawn(entity_to_despawn);
        }
    }
}

fn update_models(
    vulkan_context: &VulkanContext,
    render_context: &mut RenderContext,
    world: &mut hecs::World,
    asset: Arc<Vec<u8>>,
) {
    // First, load the asset in
    let scene =
        asset_importer::load_scene_from_glb(&asset, vulkan_context, render_context).unwrap();

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
        add_model_to_world(name, models, world, None);
    }
}

fn update_shader(
    vulkan_context: &VulkanContext,
    render_context: &mut RenderContext,
    shader_name: &str,
    asset_data: Arc<Vec<u8>>,
) {
    println!(
        "[HOTHAM_ASSET_HOT_RELOAD] STAND BACK - HOT RELOADING SHADER {shader_name} MOTHERFUCKER"
    );

    {
        let shaders = &mut render_context.shaders;
        let shader = u8_to_u32(asset_data);
        if shader_name.contains("vert") {
            shaders.vertex_shader = shader;
        } else {
            shaders.fragment_shader = shader;
        }
    }

    unsafe {
        vulkan_context.device.device_wait_idle().unwrap();
        render_context.pipeline = create_pipeline(
            vulkan_context,
            render_context.pipeline_layout,
            &render_context.render_area(),
            render_context.render_pass,
            &render_context.shaders,
        )
        .unwrap();
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
