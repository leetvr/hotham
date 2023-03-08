mod utils;
mod xpbd;

use std::time::{Duration, Instant};

use hotham::{
    asset_importer::{self, add_model_to_world},
    components::{
        hand::Handedness, physics::SharedShape, Collider, GlobalTransform, LocalTransform, Mesh,
        RigidBody, Visible,
    },
    contexts::RenderContext,
    glam::{vec2, vec3, Vec3},
    hecs::World,
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

use xpbd::{
    create_points, create_shape_constraints, resolve_collisions,
    resolve_shape_matching_constraints, Contact, ShapeConstraint,
};

const NX: usize = 10;
const NY: usize = 10;
const NZ: usize = 10;

const RESET_SIMULATION_AFTER_SECS: u64 = 10;

#[derive(Clone)]
/// Most Hotham applications will want to keep track of some sort of state.
/// However, this _simple_ scene doesn't have any, so this is just left here to let you know that
/// this is something you'd probably want to do!
struct State {
    points_curr: Vec<Vec3>,
    shape_constraints: Vec<ShapeConstraint>,
    velocities: Vec<Vec3>,
    active_collisions: Vec<Option<Contact>>,
    wall_time: Instant,
    simulation_time_epoch: Instant,
    simulation_time_hare: Instant,
    simulation_time_hound: Instant,
    dt: f32,
    acc: Vec3,             // Gravity or such
    shape_compliance: f32, // Inverse of physical stiffness

    mesh: Option<Mesh>,
}

impl Default for State {
    fn default() -> Self {
        let points_curr = create_default_points();
        let shape_constraints = create_shape_constraints(&points_curr, NX, NY, NZ);
        let velocities = vec![vec3(0.0, 0.0, 0.0); points_curr.len()];
        let mut active_collisions = Vec::<Option<Contact>>::new();
        active_collisions.resize_with(points_curr.len(), Default::default);

        let dt = 0.005;
        let acc = vec3(0.0, -9.82, 0.0);
        let shape_compliance = 0.0001;

        let mesh = None;

        let wall_time = Instant::now();
        let simulation_time_epoch = wall_time;

        State {
            points_curr,
            shape_constraints,
            velocities,
            active_collisions,
            simulation_time_hare: simulation_time_epoch,
            simulation_time_hound: simulation_time_epoch,
            wall_time,
            simulation_time_epoch,
            dt,
            acc,
            shape_compliance,
            mesh,
        }
    }
}

fn create_default_points() -> Vec<Vec3> {
    create_points(vec3(0.0, 2.0, 0.5), vec3(0.5, 0.5, 0.5), NX, NY, NZ)
}

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
pub fn main() {
    println!("[HOTHAM_SIMPLE_SCENE] MAIN!");
    real_main().expect("Error running app!");
    println!("[HOTHAM_SIMPLE_SCENE] FINISHED! Goodbye!");
}

pub fn real_main() -> HothamResult<()> {
    let mut engine = Engine::new();
    let mut state = Default::default();
    init(&mut engine, &mut state)?;

    while let Ok(tick_data) = engine.update() {
        tick(tick_data, &mut engine, &mut state);
        engine.finish()?;
    }

    Ok(())
}

fn tick(tick_data: TickData, engine: &mut Engine, state: &mut State) {
    let time_now = Instant::now();
    let time_passed = time_now.saturating_duration_since(state.wall_time);
    state.wall_time = time_now;
    if tick_data.current_state == xr::SessionState::FOCUSED {
        state.simulation_time_hare += time_passed;
        auto_reset_system(state);
        xpbd_system(state);
        hands_system(engine);
        grabbing_system(engine);
        physics_system(engine);
        animation_system(engine);
        update_global_transform_system(engine);
        update_global_transform_with_parent_system(engine);
        skinning_system(engine);
        debug_system(engine);
    }
    if let Some(mesh) = &state.mesh {
        update_mesh(mesh, &mut engine.render_context, &state.points_curr);
    }
    rendering_system(engine, tick_data.swapchain_image_index);
}

fn auto_reset_system(state: &mut State) {
    if state
        .simulation_time_hound
        .duration_since(state.simulation_time_epoch)
        >= Duration::new(RESET_SIMULATION_AFTER_SECS, 0)
    {
        state.simulation_time_hound = state.simulation_time_epoch;
        state.simulation_time_hare = state.simulation_time_epoch;
        state.points_curr = create_default_points();
        state.velocities.iter_mut().for_each(|v| *v = Vec3::ZERO);
    }
}

fn xpbd_system(state: &mut State) {
    let timestep = Duration::from_nanos((state.dt * 1_000_000_000.0) as _);
    while state.simulation_time_hound + timestep < state.simulation_time_hare {
        state.simulation_time_hound += timestep;
        xpbd_substep(state);
    }
}

fn xpbd_substep(state: &mut State) {
    // Update velocities
    for vel in &mut state.velocities {
        *vel += state.acc * state.dt;
    }

    // Predict new positions
    let mut points_next = state
        .points_curr
        .iter()
        .zip(&state.velocities)
        .map(|(&curr, &vel)| curr + vel * state.dt)
        .collect::<Vec<_>>();

    // Resolve collisions
    resolve_collisions(&mut points_next, &mut state.active_collisions);

    // TODO: Resolve distance constraints

    // Resolve shape matching constraints
    resolve_shape_matching_constraints(
        &mut points_next,
        &state.shape_constraints,
        state.shape_compliance,
        state.dt,
    );

    // Update velocities
    state.velocities = points_next
        .iter()
        .zip(&state.points_curr)
        .map(|(&next, &curr)| (next - curr) / state.dt)
        .collect::<Vec<_>>();

    state.points_curr = points_next;
}

fn init(engine: &mut Engine, state: &mut State) -> Result<(), hotham::HothamError> {
    let render_context = &mut engine.render_context;
    let vulkan_context = &mut engine.vulkan_context;
    let world = &mut engine.world;

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
    add_model_to_world("Cube", &models, world, None);

    state.mesh = Some(create_mesh(render_context, world, &state.points_curr));

    Ok(())
}

fn add_helmet(models: &std::collections::HashMap<String, World>, world: &mut World) {
    let helmet = add_model_to_world("Damaged Helmet", models, world, None)
        .expect("Could not find Damaged Helmet");

    {
        let mut local_transform = world.get::<&mut LocalTransform>(helmet).unwrap();
        local_transform.translation.z = -1.;
        local_transform.translation.y = 1.4;
        local_transform.scale = [0.5, 0.5, 0.5].into();
    }

    let collider = Collider::new(SharedShape::ball(0.35));

    world
        .insert(helmet, (collider, RigidBody::default()))
        .unwrap();
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
    update_mesh(&mesh, render_context, &points);
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
