use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

pub mod systems;

use hotham::{
    asset_importer::{self, add_model_to_world},
    components::{Mesh, Transform, TransformMatrix, Visible},
    hecs::{With, World},
    nalgebra::{UnitQuaternion, Vector3},
    rendering::{
        material::Material,
        mesh_data::MeshData,
        primitive::{calculate_bounding_sphere, Primitive},
        vertex::Vertex,
    },
    resources::RenderContext,
    schedule_functions::physics_step,
    systems::{
        animation_system, collision_system, grabbing_system, hands_system,
        rendering::rendering_system, skinning::skinning_system,
        update_parent_transform_matrix_system, update_rigid_body_transforms_system,
        update_transform_matrix_system, Queries,
    },
    xr, Engine, HothamResult, TickData,
};
use systems::setup_cubes;

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
pub fn main() {
    println!("[HOTHAM_STRESS_TEST] MAIN!");
    real_main().expect("Error running app!");
    println!("[HOTHAM_STRESS_TEST] FINISHED! Goodbye!");
}

pub fn real_main() -> HothamResult<()> {
    let mut engine = Engine::new();
    let test = StressTest::ManyCubes;
    let (world, models) = init(&mut engine, &test);
    let queries = Default::default();
    let timer = Default::default();

    let mut tick_props = TickProps {
        engine,
        world,
        models,
        queries,
        timer,
        test,
    };

    while let Ok(tick_data) = tick_props.engine.update() {
        tick(&mut tick_props, tick_data);
        tick_props.engine.finish()?;
    }

    Ok(())
}

pub struct Timer {
    start_time: Instant,
    last_frame_time: Instant,
    timer: Duration,
}

impl Default for Timer {
    fn default() -> Self {
        Self {
            start_time: Instant::now(),
            last_frame_time: Instant::now(),
            timer: Default::default(),
        }
    }
}

impl Timer {
    pub fn tick(&mut self) -> bool {
        let now = Instant::now();
        let delta = now - self.last_frame_time;
        self.last_frame_time = now;
        self.timer += delta;

        if self.timer.as_secs_f32() >= 1.0 {
            self.timer = Default::default();
            return true;
        }

        false
    }

    pub fn total_time(&self) -> Duration {
        Instant::now() - self.start_time
    }
}

#[allow(dead_code)]
/// Used to select which test to run
pub enum StressTest {
    /// Generate one cube per second
    ManyCubes,
    /// Display an additional model each second
    ManyHelmets,
    /// Create a dynamic mesh and increase the number of vertices each second
    ManyVertices,
    /// Load the New Sponza scene into the engine
    Sponza,
}

fn init(engine: &mut Engine, test: &StressTest) -> (World, HashMap<String, World>) {
    let render_context = &mut engine.render_context;
    let vulkan_context = &mut engine.vulkan_context;
    let mut world = World::default();

    let models = match test {
        StressTest::ManyCubes => {
            let glb_buffers: Vec<&[u8]> = vec![include_bytes!("../../../test_assets/cube.glb")];
            let models =
                asset_importer::load_models_from_glb(&glb_buffers, vulkan_context, render_context)
                    .unwrap();

            let resolution = 34; // 42,875 cubes

            setup_cubes(&mut world, resolution, &models);

            models
        }
        StressTest::ManyHelmets => {
            #[cfg(target_os = "android")]
            let glb_buffers: Vec<&[u8]> = vec![include_bytes!(
                "../../../test_assets/damaged_helmet_squished.glb"
            )];
            #[cfg(not(target_os = "android"))]
            let glb_buffers: Vec<&[u8]> =
                vec![include_bytes!("../../../test_assets/damaged_helmet.glb")];
            let models =
                asset_importer::load_models_from_glb(&glb_buffers, vulkan_context, render_context)
                    .unwrap();

            for _ in 0..20 {
                let e = add_model_to_world("Damaged Helmet", &models, &mut world, None)
                    .expect("Could not find cube?");
                let mut t = world.get_mut::<Transform>(e).unwrap();
                t.rotation = UnitQuaternion::from_axis_angle(
                    &Vector3::x_axis(),
                    std::f32::consts::FRAC_PI_2,
                );
            }
            models
        }
        StressTest::ManyVertices => {
            create_mesh(render_context, &mut world);
            Default::default()
        }
        StressTest::Sponza => {
            let file = std::fs::read("test_assets/sponza.glb").unwrap();
            let glb_buffers: Vec<&[u8]> = vec![&file];
            let models =
                asset_importer::load_models_from_glb(&glb_buffers, vulkan_context, render_context)
                    .unwrap();
            for name in models.keys() {
                add_model_to_world(name, &models, &mut world, None);
            }
            models
        }
    };

    (world, models)
}

struct TickProps<'a> {
    engine: Engine,
    world: World,
    models: HashMap<String, World>,
    queries: Queries<'a>,
    timer: Timer,
    test: StressTest,
}

fn tick(tick_props: &mut TickProps, tick_data: TickData) {
    let engine = &mut tick_props.engine;
    let world = &mut tick_props.world;
    let queries = &mut tick_props.queries;
    let timer = &mut tick_props.timer;
    let models = &tick_props.models;

    let xr_context = &mut engine.xr_context;
    let render_context = &mut engine.render_context;
    let physics_context = &mut engine.physics_context;

    if tick_data.current_state == xr::SessionState::FOCUSED {
        hands_system(&mut queries.hands_query, world, xr_context, physics_context);
        grabbing_system(&mut queries.grabbing_query, world, physics_context);
        physics_step(physics_context);
        collision_system(&mut queries.collision_query, world, physics_context);
        update_rigid_body_transforms_system(
            &mut queries.update_rigid_body_transforms_query,
            world,
            physics_context,
        );

        match &tick_props.test {
            // StressTest::ManyCubes => rotate_models(world, timer.total_time().as_secs_f32()),
            StressTest::ManyHelmets => model_system(world, models, timer, "Damaged Helmet"),
            StressTest::ManyVertices => subdivide_mesh_system(world, render_context, timer),
            _ => {}
        }

        animation_system(&mut queries.animation_query, world);
        update_transform_matrix_system(&mut queries.update_transform_matrix_query, world);
        update_parent_transform_matrix_system(
            &mut queries.parent_query,
            &mut queries.roots_query,
            world,
        );
        skinning_system(&mut queries.skins_query, world, render_context);
    }

    // Rendering!
    let vulkan_context = &engine.vulkan_context;
    let views = xr_context.update_views();
    rendering_system(
        &mut queries.rendering_query,
        world,
        vulkan_context,
        render_context,
        views,
        tick_data.swapchain_image_index,
    );
}

fn subdivide_mesh_system(world: &mut World, render_context: &mut RenderContext, timer: &mut Timer) {
    if !timer.tick() {
        return;
    }

    // Get the mesh
    let mesh = world.query_mut::<&mut Mesh>().into_iter().next().unwrap().1;

    // Calculate the current step.
    let step = timer.total_time().as_secs() * 10;
    update_mesh(step as _, mesh, render_context);
}

fn model_system(
    world: &mut World,
    models: &HashMap<String, World>,
    timer: &mut Timer,
    model_name: &str,
) {
    if timer.tick() {
        add_model_to_world(model_name, models, world, None).expect("Could not find object?");
        rearrange_models(world);
    }

    rotate_models(world, timer.total_time().as_secs_f32());
}

fn rotate_models(world: &mut World, total_time: f32) {
    for (_, transform) in world.query_mut::<With<Mesh, &mut Transform>>() {
        transform.rotation =
            UnitQuaternion::from_euler_angles(90.0_f32.to_radians(), total_time.sin() * 2., 0.);
    }
}

fn rearrange_models(world: &mut World) {
    let query = world.query_mut::<With<Mesh, &mut Transform>>();
    let query_iter = query.into_iter();
    let num_models = query_iter.len() as f32;

    let column_size = num_models.sqrt() as usize;
    let scale = 2. / column_size as f32;
    let half_column_size = column_size as f32 / 2.;
    let mut row = 0;
    let mut column = 0;

    for (_, transform) in query_iter {
        if column >= column_size {
            column = 0;
            row += 1;
        }

        transform.translation.x = (column as f32) - half_column_size;
        transform.translation.y = (row as f32) - 0.5;
        transform.translation.z = -4.0;

        transform.scale = Vector3::repeat(scale);

        column += 1;
    }

    println!("[HOTHAM_STRESS_TEST] There are now {} models", num_models);
}

fn create_mesh(render_context: &mut RenderContext, world: &mut World) {
    let material_id = unsafe {
        render_context
            .resources
            .materials_buffer
            .push(&Material::unlit_white())
    };
    let mesh = Mesh::new(
        MeshData::new(vec![Primitive {
            material_id,
            ..Default::default()
        }]),
        render_context,
    );
    update_mesh(1, &mesh, render_context);
    let transform = Transform {
        translation: [0., 1., -1.].into(),
        ..Default::default()
    };

    world.spawn((Visible {}, mesh, transform, TransformMatrix::default()));
}

fn update_mesh(step: usize, mesh: &Mesh, render_context: &mut RenderContext) {
    let mut vertices = vec![];
    let mut indices = vec![];

    let vertex_offset = 1.0 / step as f32;

    let mut n = 0;
    for row in 0..step {
        for column in 0..step {
            {
                let row = row as f32;
                let col = column as f32;
                let x = col * vertex_offset;
                let y = row * vertex_offset;
                vertices.push(vertex(x, y + vertex_offset));
                vertices.push(vertex(x, y));
                vertices.push(vertex(x + vertex_offset, y));
                vertices.push(vertex(x + vertex_offset, y + vertex_offset));
            }

            let index_offset = (n * 4) as u32;
            indices.push(index_offset);
            indices.push(index_offset + 1);
            indices.push(index_offset + 2);
            indices.push(index_offset);
            indices.push(index_offset + 2);
            indices.push(index_offset + 3);
            n += 1;
        }
    }

    let mesh = render_context
        .resources
        .mesh_data
        .get_mut(mesh.handle)
        .unwrap();
    mesh.primitives[0].indices_count = indices.len() as _;
    mesh.primitives[0].bounding_sphere = calculate_bounding_sphere(&vertices);

    // This is *really* nasty, but we can get away with it as we won't have any other meshes in the scene.
    // In the real world, this would potentially obliterate existing meshes as we're overwriting the shared buffers.
    // DON'T DO THIS in a real application!
    unsafe {
        render_context.resources.index_buffer.overwrite(&indices);
        render_context.resources.vertex_buffer.overwrite(&vertices);
    }

    println!(
        "There are now {} vertices and {} indices",
        vertices.len(),
        indices.len()
    );
}

fn vertex(x: f32, y: f32) -> Vertex {
    Vertex {
        position: [x, y, -1.0].into(),
        ..Default::default()
    }
}
