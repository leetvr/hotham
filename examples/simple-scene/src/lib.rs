mod audio_player;
mod utils;
mod xpbd_audio_bridge;
mod xpbd_collisions;
mod xpbd_shape_constraints;
mod xpbd_substep;

use std::time::{Duration, Instant};

use hotham::{
    asset_importer::{self, add_model_to_world},
    components::{
        hand::Handedness,
        physics::{BodyType, SharedShape},
        Collider, GlobalTransform, Grabbable, LocalTransform, Mesh, RigidBody, Visible,
    },
    contexts::{InputContext, RenderContext},
    glam::{vec2, vec3, Vec3},
    hecs::{self, Without, World},
    na,
    rendering::{
        material::Material,
        mesh_data::MeshData,
        primitive::{calculate_bounding_sphere, Primitive},
        vertex::Vertex,
    },
    systems::{
        animation_system, debug::debug_system, grabbing_system, hands::add_hand, hands_system,
        physics_system, rendering::rendering_system, skinning::skinning_system,
        update_global_transform_system, update_global_transform_with_parent_system,
    },
    xr, Engine, HothamResult, TickData,
};
use hotham_examples::navigation::{navigation_system, State as NavigationState};

use inline_tweak::tweak;
use nalgebra::DVector;
use xpbd_audio_bridge::{AudioSimulationUpdate, AudioState};
use xpbd_collisions::Contact;
use xpbd_collisions::XpbdCollisions;
use xpbd_shape_constraints::{create_points, create_shape_constraints, ShapeConstraint};
use xpbd_substep::xpbd_substep;

use crate::xpbd_substep::SimulationParams;

const NX: usize = 10;
const NY: usize = 10;
const NZ: usize = 10;

/// Most Hotham applications will want to keep track of some sort of state.
/// However, this _simple_ scene doesn't have any, so this is just left here to let you know that
/// this is something you'd probably want to do!
struct State {
    points_curr: Vec<Vec3>,
    velocities: Vec<Vec3>,
    shape_constraints: Vec<ShapeConstraint>,
    audio_emitter_indices: Vec<usize>,
    wall_time: Instant,
    simulation_time_epoch: Instant,
    simulation_time_hare: Instant,
    simulation_time_hound: Instant,
    mesh: Option<Mesh>,
    navigation: NavigationState,
    audio_state: AudioState,
    audio_sample_counter: u64,
}

impl Default for State {
    fn default() -> Self {
        let points_curr = create_default_points();
        let shape_constraints = create_shape_constraints(&points_curr, NX, NY, NZ);
        let velocities = vec![vec3(0.0, 0.0, 0.0); points_curr.len()];

        let mesh = None;

        let wall_time = Instant::now();
        let simulation_time_epoch = wall_time;

        // Pick the corners as audio emitters
        let ix1 = 0;
        let ix2 = NX - 1;
        let iy1 = 0;
        let iy2 = NY - 1;
        let iz1 = 0;
        let iz2 = NX - 1;

        let audio_emitter_indices = vec![
            iz1 * NX * NY + iy1 * NX + ix1,
            iz1 * NX * NY + iy1 * NX + ix2,
            iz1 * NX * NY + iy2 * NX + ix1,
            iz1 * NX * NY + iy2 * NX + ix2,
            iz2 * NX * NY + iy1 * NX + ix1,
            iz2 * NX * NY + iy1 * NX + ix2,
            iz2 * NX * NY + iy2 * NX + ix1,
            iz2 * NX * NY + iy2 * NX + ix2,
        ];
        let num_points = audio_emitter_indices.len();

        State {
            points_curr,
            velocities,
            shape_constraints,
            audio_emitter_indices,
            simulation_time_hare: simulation_time_epoch,
            simulation_time_hound: simulation_time_epoch,
            wall_time,
            simulation_time_epoch,
            mesh,
            navigation: Default::default(),
            audio_state: AudioState::init_audio(num_points, simulation_time_epoch).unwrap(),
            audio_sample_counter: 0,
        }
    }
}

fn create_default_points() -> Vec<Vec3> {
    create_points(
        vec3(tweak!(0.0), tweak!(2.0), tweak!(-0.5)),
        vec3(0.5, 0.5, 0.5),
        NX,
        NY,
        NZ,
    )
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
    let mut state = Default::default();
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
        state.simulation_time_hare += time_passed.min(Duration::from_millis(100));
        simulation_reset_system(&engine.input_context, state);
        hands_system(engine);
        grabbing_system(engine);
        physics_system(engine);
        animation_system(engine);
        navigation_system(engine, &mut state.navigation);
        update_global_transform_system(engine);
        update_global_transform_with_parent_system(engine);
        skinning_system(engine);
        debug_system(engine);
        xpbd_system(engine, state);
        log_audio_system(state);
    }
    if let Some(mesh) = &state.mesh {
        update_mesh(mesh, &mut engine.render_context, &state.points_curr);
    }
    rendering_system(engine, tick_data.swapchain_image_index);
}

fn simulation_reset_system(input_context: &InputContext, state: &mut State) {
    if input_context.left.menu_button_just_pressed() {
        state.simulation_time_hound = state.simulation_time_epoch;
        state.simulation_time_hare = state.simulation_time_epoch;
        state.points_curr = create_default_points();
        state.velocities.iter_mut().for_each(|v| *v = Vec3::ZERO);
    }
}

fn xpbd_system(engine: &mut Engine, state: &mut State) {
    puffin::profile_function!();
    let simulation_params = {
        puffin::profile_scope!("simulation params");
        SimulationParams {
            dt: tweak!(0.001),
            acc: vec3(0.0, -9.82, 0.0),
            particle_mass: tweak!(0.01),
            shape_compliance: tweak!(0.00001), // Inverse of physical stiffness
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
        active_collisions.resize_with(state.points_curr.len(), Default::default);
        command_buffer.insert_one(entity, XpbdCollisions { active_collisions });
    }
    command_buffer.run_on(&mut engine.world);

    let timestep = Duration::from_nanos((dt * 1_000_000_000.0) as _);
    while state.simulation_time_hound + timestep < state.simulation_time_hare {
        state.simulation_time_hound += timestep;
        xpbd_substep(
            &mut engine.world,
            &mut state.velocities,
            &mut state.points_curr,
            &mut state.shape_constraints,
            &simulation_params,
        );
        send_xpbd_state_to_audio(
            &state.points_curr,
            &state.velocities,
            &state.audio_emitter_indices,
            state.simulation_time_hound,
            &mut state.audio_state,
        );
    }
}

fn send_xpbd_state_to_audio(
    points_curr: &[Vec3],
    velocities: &[Vec3],
    audio_emitter_indices: &[usize],
    simulation_time: Instant,
    audio_state: &mut AudioState,
) {
    let num_emitters = audio_emitter_indices.len();
    let mut state_vector = DVector::<f32>::zeros(num_emitters * 3 * 2);
    for (i, &ip) in audio_emitter_indices.iter().enumerate() {
        state_vector[i * 3] = points_curr[i].x;
        state_vector[i * 3 + 1] = points_curr[i].y;
        state_vector[i * 3 + 2] = points_curr[i].z;
        state_vector[(num_emitters + i) * 3] = velocities[i].x;
        state_vector[(num_emitters + i) * 3 + 1] = velocities[i].y;
        state_vector[(num_emitters + i) * 3 + 2] = velocities[i].z;
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

    state.mesh = Some(create_mesh(render_context, world, &state.points_curr));

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

fn create_mesh(render_context: &mut RenderContext, world: &mut World, points: &[Vec3]) -> Mesh {
    const N: u32 = 10_u32;
    const M: u32 = N - 1;
    const NUM_POINTS: usize = (N * N * N) as _;
    assert_eq!(points.len(), NUM_POINTS);
    const NUM_VERTICES: usize = (6 * N * N) as _;
    const NUM_INDICES: usize = (6 * M * M * 2) as _;
    let positions: Vec<Vec3> = vec![Default::default(); NUM_VERTICES];
    let vertices: Vec<Vertex> = vec![Default::default(); NUM_VERTICES];
    let mut indices: Vec<u32> = vec![Default::default(); NUM_INDICES];

    for side in 0..6 {
        for i in 0..M {
            for j in 0..M {
                indices.push(side * N * N + i * N + j);
                indices.push(side * N * N + i * N + j + 1);
                indices.push(side * N * N + i * N + j + 1 + N);
                indices.push(side * N * N + i * N + j);
                indices.push(side * N * N + i * N + j + 1 + N);
                indices.push(side * N * N + i * N + j + N);
            }
        }
    }

    let material_id = unsafe {
        render_context
            .resources
            .materials_buffer
            .push(&Material::gltf_default())
    };
    let mesh = Mesh::new(
        MeshData::new(vec![Primitive::new(
            positions.as_slice(),
            vertices.as_slice(),
            indices.as_slice(),
            material_id,
            render_context,
        )]),
        render_context,
    );
    update_mesh(&mesh, render_context, points);
    let local_transform = LocalTransform {
        translation: [0., 0., 0.].into(),
        ..Default::default()
    };

    world.spawn((
        Visible {},
        mesh.clone(),
        local_transform,
        GlobalTransform::default(),
    ));

    mesh
}

fn update_mesh(mesh: &Mesh, render_context: &mut RenderContext, points: &[Vec3]) {
    const N: i32 = 10_i32;
    const M: i32 = N - 1;
    const NUM_VERTICES: usize = (6 * N * N) as _;
    let mut positions: Vec<Vec3> = Vec::<Vec3>::with_capacity(NUM_VERTICES);
    let mut vertices: Vec<Vertex> = Vec::<Vertex>::with_capacity(NUM_VERTICES);

    for side in 0..6 {
        for i in 0..N {
            for j in 0..N {
                let (x, y, z, dxdi, dydi, dzdi, dxdj, dydj, dzdj) = match side {
                    0 => (M, i, M - j, 0, 1, 0, 0, 0, -1),
                    1 => (j, M, M - i, 0, 0, -1, 1, 0, 0),
                    2 => (j, i, M, 0, 1, 0, 1, 0, 0),
                    3 => (0, j, M - i, 0, 0, -1, 0, 1, 0),
                    4 => (j, 0, i, 0, 0, 1, 1, 0, 0),
                    5 => (M - j, i, 0, 0, 1, 0, -1, 0, 0),
                    i32::MIN..=-1_i32 | 6_i32..=i32::MAX => todo!(),
                };
                let center = points[(z * N * N + y * N + x) as usize];
                positions.push(center);
                let x0 = (x - dxdi).clamp(0, M);
                let y0 = (y - dydi).clamp(0, M);
                let z0 = (z - dzdi).clamp(0, M);
                let x1 = (x + dxdi).clamp(0, M);
                let y1 = (y + dydi).clamp(0, M);
                let z1 = (z + dzdi).clamp(0, M);
                let x2 = (x - dxdj).clamp(0, M);
                let y2 = (y - dydj).clamp(0, M);
                let z2 = (z - dzdj).clamp(0, M);
                let x3 = (x + dxdj).clamp(0, M);
                let y3 = (y + dydj).clamp(0, M);
                let z3 = (z + dzdj).clamp(0, M);
                let p_down = points[(z0 * N * N + y0 * N + x0) as usize];
                let p_up = points[(z1 * N * N + y1 * N + x1) as usize];
                let p_left = points[(z2 * N * N + y2 * N + x2) as usize];
                let p_right = points[(z3 * N * N + y3 * N + x3) as usize];
                let normal = (p_right - p_left).cross(p_up - p_down).normalize_or_zero();
                let texture_coords = vec2(j as f32 / M as f32, i as f32 / M as f32);
                vertices.push(Vertex {
                    normal,
                    texture_coords,
                    ..Default::default()
                });
            }
        }
    }

    let mesh = render_context
        .resources
        .mesh_data
        .get_mut(mesh.handle)
        .unwrap();
    mesh.primitives[0].bounding_sphere = calculate_bounding_sphere(&positions);

    unsafe {
        std::ptr::copy_nonoverlapping(
            positions.as_ptr(),
            render_context
                .resources
                .position_buffer
                .memory_address
                .as_ptr()
                .offset(mesh.primitives[0].vertex_buffer_offset as _),
            positions.len(),
        );
        std::ptr::copy_nonoverlapping(
            vertices.as_ptr(),
            render_context
                .resources
                .vertex_buffer
                .memory_address
                .as_ptr()
                .offset(mesh.primitives[0].vertex_buffer_offset as _),
            vertices.len(),
        );
    }
}

fn log_audio_system(state: &mut State) {
    let sample_rate = state.audio_state.audio_player.config.sample_rate().0 as u64;
    let samples_per_log = sample_rate / 10;
    while let Some(entry) = state.audio_state.audio_player.get_audio_history_entry() {
        if state.audio_sample_counter % samples_per_log == 0 {
            println!("{entry:?}");
        }
        state.audio_sample_counter += 1;
    }
}
