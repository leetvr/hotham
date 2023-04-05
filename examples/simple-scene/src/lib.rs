mod audio_player;
mod utils;
mod xpbd_audio_bridge;
mod xpbd_collisions;
mod xpbd_rerun;
mod xpbd_shape_constraints;
mod xpbd_state;
mod xpbd_substep;

use std::time::{Duration, Instant};

use hotham::{
    asset_importer::{self, add_model_to_world},
    components::{
        hand::Handedness,
        physics::{BodyType, SharedShape},
        stage, Collider, GlobalTransform, Grabbable, LocalTransform, RigidBody,
    },
    glam::{dvec3, Affine3A, DVec3},
    hecs::{self, Without, World},
    na,
    systems::{
        animation_system, debug::debug_system, grabbing_system, hands::add_hand, hands_system,
        physics_system, rendering::rendering_system, skinning::skinning_system,
        update_global_transform_system, update_global_transform_with_parent_system,
    },
    xr, Engine, HothamResult, TickData,
};
use hotham_examples::navigation::{navigation_system, State as NavigationState};

use inline_tweak::tweak;
use nalgebra::{DVector, Quaternion, Translation3, UnitQuaternion};
use xpbd_audio_bridge::{AudioSimulationUpdate, AudioState};
use xpbd_collisions::Contact;
use xpbd_collisions::XpbdCollisions;
use xpbd_rerun::init_rerun_session;
use xpbd_shape_constraints::create_points;
use xpbd_state::XpbdState;
use xpbd_substep::xpbd_substep;

use crate::{
    audio_player::ListenerPose,
    xpbd_rerun::{send_colliders_to_rerun, send_xpbd_state_to_rerun},
    xpbd_substep::SimulationParams,
};

const NX: usize = 5;
const NY: usize = 5;
const NZ: usize = 5;

fn get_default_center() -> DVec3 {
    dvec3(tweak!(0.0), tweak!(2.0), tweak!(-0.5))
}

fn get_default_size() -> DVec3 {
    dvec3(0.25, 0.25, 0.25)
}

pub struct FrameStartTransform(pub Affine3A);

pub struct InterpolatedTransform(pub Affine3A);

impl InterpolatedTransform {
    /// Convenience function to convert the [`GlobalTransform`] into a [`rapier3d::na::Isometry3`]
    pub fn to_isometry(&self) -> hotham::na::Isometry3<f32> {
        hotham::util::isometry_from_affine(&self.0)
    }
}

fn store_transforms_pre_update_system(engine: &mut Engine) {
    puffin::profile_function!();

    let mut command_buffer = hecs::CommandBuffer::new();
    for (entity, global_transform) in engine
        .world
        .query::<Without<&GlobalTransform, &FrameStartTransform>>()
        .iter()
    {
        command_buffer.insert_one(entity, FrameStartTransform(global_transform.0));
    }
    for (entity, global_transform) in engine
        .world
        .query::<Without<&GlobalTransform, &InterpolatedTransform>>()
        .iter()
    {
        command_buffer.insert_one(entity, InterpolatedTransform(global_transform.0));
    }
    command_buffer.run_on(&mut engine.world);

    for (_, (global_transform, start_transform)) in engine
        .world
        .query_mut::<(&GlobalTransform, &mut FrameStartTransform)>()
        .into_iter()
    {
        start_transform.0 = global_transform.0;
    }
}

/// Most Hotham applications will want to keep track of some sort of state.
/// However, this _simple_ scene doesn't have any, so this is just left here to let you know that
/// this is something you'd probably want to do!
struct State {
    wall_time: Instant,
    navigation: NavigationState,
    audio_state: AudioState,
    audio_sample_counter: u64,
    xpbd_state: XpbdState,
    rr_session: Option<rerun::Session>,
}

impl Default for State {
    fn default() -> Self {
        let wall_time = Instant::now();
        let xpbd_state = XpbdState::new(
            get_default_center(),
            get_default_size(),
            NX,
            NY,
            NZ,
            wall_time,
        );
        let num_points = xpbd_state.audio_emitter_indices.len();
        State {
            wall_time,
            navigation: Default::default(),
            audio_state: AudioState::init_audio(num_points, wall_time).unwrap(),
            audio_sample_counter: 0,
            xpbd_state,
            rr_session: None,
        }
    }
}

pub fn start_puffin_server() {
    puffin::set_scopes_on(true); // Tell puffin to collect data

    match puffin_http::Server::new("0.0.0.0:8585") {
        Ok(puffin_server) => {
            eprintln!("Run:  cargo install puffin_viewer && puffin_viewer --url 127.0.0.1:8585");

            // We can store the server if we want, but in this case we just want
            // it to keep running. Dropping it closes the server, so let's not drop it!
            #[allow(clippy::mem_forget)]
            std::mem::forget(puffin_server);
        }
        Err(err) => {
            eprintln!("Failed to start puffin server: {}", err);
        }
    };
}

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
pub fn main() {
    println!("[HOTHAM_SIMPLE_SCENE] MAIN!");
    real_main().expect("Error running app!");
    println!("[HOTHAM_SIMPLE_SCENE] FINISHED! Goodbye!");
}

pub fn real_main() -> HothamResult<()> {
    start_puffin_server();
    let mut engine = Engine::new();
    let mut state = State::default();
    state.rr_session = init_rerun_session().ok();
    init(&mut engine, &mut state)?;

    while let Ok(tick_data) = engine.update() {
        puffin::GlobalProfiler::lock().new_frame();
        tick(tick_data, &mut engine, &mut state);
        engine.finish()?;
    }

    Ok(())
}

fn tick(tick_data: TickData, engine: &mut Engine, state: &mut State) {
    puffin::profile_function!();
    let time_now = Instant::now();
    let time_passed = time_now.saturating_duration_since(state.wall_time);
    state.wall_time = time_now;
    if tick_data.current_state == xr::SessionState::FOCUSED {
        store_transforms_pre_update_system(engine);
        simulation_reset_system(engine, state);
        hands_system(engine);
        grabbing_system(engine);
        physics_system(engine);
        animation_system(engine);
        navigation_system(engine, &mut state.navigation);
        update_global_transform_system(engine);
        update_global_transform_with_parent_system(engine);
        skinning_system(engine);
        debug_system(engine);
        xpbd_system(engine, state, time_passed);
        update_listener_system(engine, state);
        log_audio_system(state);
    }
    if let Some(mesh) = &state.xpbd_state.mesh {
        xpbd_mesh::update_mesh(
            mesh,
            &mut engine.render_context,
            &state.xpbd_state.points_curr,
            NX,
        );
    }
    rendering_system(engine, tick_data.swapchain_image_index);
}

fn simulation_reset_system(engine: &mut Engine, state: &mut State) {
    let input_context = &engine.input_context;
    if input_context.left.menu_button_just_pressed() || input_context.right.a_button_just_pressed()
    {
        state.xpbd_state.simulation_time_hound = state.xpbd_state.simulation_time_epoch;
        state.xpbd_state.simulation_time_hare = state.xpbd_state.simulation_time_epoch;
        state.xpbd_state.points_curr =
            create_points(get_default_center(), get_default_size(), NX, NY, NZ);
        state
            .xpbd_state
            .velocities
            .iter_mut()
            .for_each(|v| *v = DVec3::ZERO);
        for c in &mut state.xpbd_state.shape_constraints {
            c.cached_rot = Default::default();
        }
    }
}

fn xpbd_system(engine: &mut Engine, state: &mut State, time_passed: Duration) {
    puffin::profile_function!();
    let simulation_params = {
        puffin::profile_scope!("simulation params");
        SimulationParams {
            dt: tweak!(0.001),
            acc: dvec3(0.0, -9.82, 0.0),
            particle_mass: tweak!(0.01),
            shape_compliance: tweak!(0.0001), // Inverse of physical stiffness
            shape_damping: tweak!(100.0), // Linear damping towards rigid body motion, fraction of speed per second
            stiction_factor: tweak!(1.3), // Maximum tangential correction per correction along normal.
        }
    };
    let dt = simulation_params.dt;

    let mut command_buffer = hecs::CommandBuffer::new();

    // Create XpbdCollisions if they are missing
    for (entity, _) in engine
        .world
        .query::<Without<&Collider, &XpbdCollisions>>()
        .iter()
    {
        let mut active_collisions = Vec::<Option<Contact>>::new();
        active_collisions.resize_with(state.xpbd_state.points_curr.len(), Default::default);
        command_buffer.insert_one(entity, XpbdCollisions { active_collisions });
    }
    command_buffer.run_on(&mut engine.world);

    let substep = Duration::from_secs_f64(dt);
    state.xpbd_state.simulation_time_hare += time_passed.min(Duration::from_millis(100));

    let time_start = state.xpbd_state.simulation_time_hound;
    let all_substeps_secs = {
        let mut time_end = state.xpbd_state.simulation_time_hound;
        while time_end + substep < state.xpbd_state.simulation_time_hare {
            time_end += substep;
        }
        (time_end - time_start).as_secs_f32()
    };

    while state.xpbd_state.simulation_time_hound + substep < state.xpbd_state.simulation_time_hare {
        state.xpbd_state.simulation_time_hound += substep;
        {
            puffin::profile_scope!("Interpolate global transforms");
            let timestep_fraction = (state.xpbd_state.simulation_time_hound - time_start)
                .as_secs_f32()
                / all_substeps_secs;
            for (_, (start, end, interpolated)) in engine
                .world
                .query_mut::<(
                    &FrameStartTransform,
                    &GlobalTransform,
                    &mut InterpolatedTransform,
                )>()
                .into_iter()
            {
                // The transform is global_from_local
                let (start_scale, start_rotation, start_translation) =
                    start.0.to_scale_rotation_translation();
                let (end_scale, end_rotation, end_translation) =
                    end.0.to_scale_rotation_translation();
                interpolated.0 = Affine3A::from_scale_rotation_translation(
                    start_scale.lerp(end_scale, timestep_fraction),
                    start_rotation.slerp(end_rotation, timestep_fraction),
                    start_translation.lerp(end_translation, timestep_fraction),
                );
            }
        }

        xpbd_substep(
            &mut engine.world,
            &mut state.xpbd_state.velocities,
            &mut state.xpbd_state.points_curr,
            &mut state.xpbd_state.shape_constraints,
            &simulation_params,
        );
        send_xpbd_state_to_audio(
            &state.xpbd_state.points_curr,
            &state.xpbd_state.velocities,
            &state.xpbd_state.audio_emitter_indices,
            state.xpbd_state.simulation_time_hound,
            &mut state.audio_state,
        );

        if let Some(session) = state.rr_session.as_mut() {
            if let Err(err) = send_xpbd_state_to_rerun(
                &state.xpbd_state,
                session,
                state.xpbd_state.simulation_time_hound,
                state.xpbd_state.simulation_time_epoch,
            ) {
                eprintln!("Error sending xpbd state to rerun: {err}");
            }
            if let Err(err) = send_colliders_to_rerun(
                &engine.world,
                session,
                state.xpbd_state.simulation_time_hound,
                state.xpbd_state.simulation_time_epoch,
            ) {
                eprintln!("Error sending colliders to rerun: {err}");
            }
        }
    }
}

fn send_xpbd_state_to_audio(
    points_curr: &[DVec3],
    velocities: &[DVec3],
    audio_emitter_indices: &[usize],
    simulation_time: Instant,
    audio_state: &mut AudioState,
) {
    let num_emitters = audio_emitter_indices.len();
    let mut state_vector = DVector::<f32>::zeros(num_emitters * 3 * 2);
    for (i, &ip) in audio_emitter_indices.iter().enumerate() {
        state_vector[i * 3] = points_curr[ip].x as _;
        state_vector[i * 3 + 1] = points_curr[ip].y as _;
        state_vector[i * 3 + 2] = points_curr[ip].z as _;
        state_vector[(num_emitters + i) * 3] = velocities[ip].x as _;
        state_vector[(num_emitters + i) * 3 + 1] = velocities[ip].y as _;
        state_vector[(num_emitters + i) * 3 + 2] = velocities[ip].z as _;
    }
    // Get old states from the audio thread and drop them here to avoid deallocating memory in the audio thread.
    audio_state
        .to_audio_producer
        .push(AudioSimulationUpdate {
            state_vector,
            simulation_time,
        })
        .unwrap();
    loop {
        if audio_state.to_ui_consumer.pop().is_err() {
            break;
        }
    }
}

fn init(engine: &mut Engine, state: &mut State) -> Result<(), hotham::HothamError> {
    let render_context = &mut engine.render_context;
    let vulkan_context = &mut engine.vulkan_context;
    let world = &mut engine.world;

    add_floor(world);

    let mut glb_buffers: Vec<&[u8]> = vec![
        include_bytes!("../../../test_assets/left_hand.glb"),
        include_bytes!("../../../test_assets/right_hand.glb"),
    ];
    let models =
        asset_importer::load_models_from_glb(&glb_buffers, vulkan_context, render_context)?;
    add_hand(&models, Handedness::Left, world);
    add_hand(&models, Handedness::Right, world);

    #[cfg(target_os = "android")]
    glb_buffers.push(include_bytes!(
        "../../../test_assets/damaged_helmet_squished.glb"
    ));

    #[cfg(not(target_os = "android"))]
    glb_buffers.push(include_bytes!("../../../test_assets/damaged_helmet.glb"));

    let models =
        asset_importer::load_models_from_glb(&glb_buffers, vulkan_context, render_context)?;
    add_helmet(&models, world);
    // add_model_to_world("Cube", &models, world, None);

    state.xpbd_state.mesh = Some(xpbd_mesh::create_mesh(
        render_context,
        world,
        &state.xpbd_state.points_curr,
        NX,
    ));

    Ok(())
}

fn add_floor(world: &mut World) {
    let entity = world.reserve_entity();
    let collider = Collider::new(SharedShape::halfspace(na::Vector3::y_axis()));
    let rigid_body = RigidBody {
        body_type: BodyType::Fixed,
        ..Default::default()
    };
    world.insert(entity, (collider, rigid_body)).unwrap();
}

fn add_helmet(models: &std::collections::HashMap<String, World>, world: &mut World) {
    let helmet = add_model_to_world("Damaged Helmet", models, world, None)
        .expect("Could not find Damaged Helmet");

    {
        let mut local_transform = world.get::<&mut LocalTransform>(helmet).unwrap();
        local_transform.translation.z = -1.;
        local_transform.translation.y = 0.4;
        local_transform.scale = [0.5, 0.5, 0.5].into();
    }

    let collider = Collider::new(SharedShape::ball(0.35));

    world.insert(helmet, (collider, Grabbable {})).unwrap();
}

mod xpbd_mesh;

fn update_listener_system(engine: &mut Engine, state: &mut State) {
    puffin::profile_function!();
    let stage_from_hmd = engine.input_context.hmd.hmd_in_stage();
    let global_from_stage = stage::get_global_from_stage(&engine.world);
    let (_scale, rotation, translation) =
        (global_from_stage * stage_from_hmd).to_scale_rotation_translation();
    let global_from_listener = ListenerPose::from_parts(
        Translation3::new(translation.x, translation.y, translation.z),
        UnitQuaternion::new_unchecked(Quaternion::new(
            rotation.w, rotation.x, rotation.y, rotation.z,
        )),
    );
    state
        .audio_state
        .audio_player
        .set_listener_pose(&global_from_listener)
        .unwrap();
}

fn log_audio_system(state: &mut State) {
    // let sample_rate = state.audio_state.audio_player.config.sample_rate().0 as u64;
    // let samples_per_log = sample_rate / 100;
    // while let Some(entry) = state.audio_state.audio_player.get_audio_history_entry() {
    //     if state.audio_sample_counter % samples_per_log == 0 {
    //         println!("{entry:?}");
    //     }
    //     state.audio_sample_counter += 1;
    // }
    while state
        .audio_state
        .audio_player
        .get_audio_history_entry()
        .is_some()
    {
        state.audio_sample_counter += 1;
    }
}
