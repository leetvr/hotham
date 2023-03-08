use hotham::{
    asset_importer::{self, add_model_to_world},
    components::{
        hand::Handedness, physics::SharedShape, Collider, GlobalTransform, LocalTransform, Mesh,
        RigidBody, Visible,
    },
    contexts::RenderContext,
    glam::{vec3, Vec3},
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

#[derive(Clone, Debug, Default)]
/// Most Hotham applications will want to keep track of some sort of state.
/// However, this _simple_ scene doesn't have any, so this is just left here to let you know that
/// this is something you'd probably want to do!
struct State {}

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
pub fn main() {
    println!("[HOTHAM_SIMPLE_SCENE] MAIN!");
    real_main().expect("Error running app!");
    println!("[HOTHAM_SIMPLE_SCENE] FINISHED! Goodbye!");
}

pub fn real_main() -> HothamResult<()> {
    let mut engine = Engine::new();
    let mut state = Default::default();
    init(&mut engine)?;

    while let Ok(tick_data) = engine.update() {
        tick(tick_data, &mut engine, &mut state);
        engine.finish()?;
    }

    Ok(())
}

fn tick(tick_data: TickData, engine: &mut Engine, _state: &mut State) {
    if tick_data.current_state == xr::SessionState::FOCUSED {
        hands_system(engine);
        grabbing_system(engine);
        physics_system(engine);
        animation_system(engine);
        update_global_transform_system(engine);
        update_global_transform_with_parent_system(engine);
        skinning_system(engine);
        debug_system(engine);
    }

    rendering_system(engine, tick_data.swapchain_image_index);
}

fn init(engine: &mut Engine) -> Result<(), hotham::HothamError> {
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

    create_mesh(render_context, world);

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

fn create_mesh(render_context: &mut RenderContext, world: &mut World) {
    let positions: Vec<Vec3> = vec![Default::default(); 1000];
    let vertices: Vec<Vertex> = vec![Default::default(); 1000];
    let mut indices: Vec<u32> = Vec::<u32>::new(); // with_capacity()

    let n = 10_i32;
    let m = n - 1;
    // Loop over blocks of vertices
    for side in 0..6 {
        for i in 0..m {
            for j in 0..m {
                let (x, y, z, dxdi, dydi, dzdi, dxdj, dydj, dzdj) = match side {
                    0 => (m, i, m - j, 0, 1, 0, 0, 0, -1),
                    1 => (j, m, m - i, 0, 0, -1, 1, 0, 0),
                    2 => (j, i, m, 0, 1, 0, 1, 0, 0),
                    3 => (0, j, m - i, 0, 0, -1, 0, 1, 0),
                    4 => (j, 0, i, 0, 0, 1, 1, 0, 0),
                    5 => (m - j, i, 0, 0, 1, 0, -1, 0, 0),
                    i32::MIN..=-1_i32 | 6_i32..=i32::MAX => todo!(),
                };
                let x0 = x;
                let y0 = y;
                let z0 = z;
                let x1 = x + dxdi;
                let y1 = y + dydi;
                let z1 = z + dzdi;
                let x2 = x + dxdi + dxdj;
                let y2 = y + dydi + dydj;
                let z2 = z + dzdi + dzdj;
                let x3 = x + dxdj;
                let y3 = y + dydj;
                let z3 = z + dzdj;
                indices.push((z0 * n * n + y0 * n + x0) as _);
                indices.push((z1 * n * n + y1 * n + x1) as _);
                indices.push((z2 * n * n + y2 * n + x2) as _);
                indices.push((z0 * n * n + y0 * n + x0) as _);
                indices.push((z2 * n * n + y2 * n + x2) as _);
                indices.push((z3 * n * n + y3 * n + x3) as _);
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
    update_mesh(&mesh, render_context);
    let local_transform = LocalTransform {
        translation: [0., 1., -1.].into(),
        ..Default::default()
    };

    world.spawn((
        Visible {},
        mesh,
        local_transform,
        GlobalTransform::default(),
    ));
}

fn update_mesh(mesh: &Mesh, render_context: &mut RenderContext) {
    let n = 10;
    let scale = 0.1;
    let mut positions = Vec::with_capacity(n * n * n);
    for i in 0..n {
        for j in 0..n {
            for k in 0..n {
                positions.push(vec3(i as _, j as _, k as _) * scale);
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
    }
}
