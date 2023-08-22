use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

pub mod systems;

use hotham::{
    asset_importer::{self, add_model_to_world},
    components::{GlobalTransform, Mesh, Visible},
    contexts::RenderContext,
    glam::{vec3, Affine3A, EulerRot, Quat, Vec3},
    hecs::{With, World},
    rendering::{
        light::Light,
        material::Material,
        mesh_data::MeshData,
        primitive::{calculate_bounding_sphere, Primitive},
    },
    systems::{
        animation_system, debug::debug_system, grabbing_system, hands_system, physics_system,
        rendering::rendering_system, skinning::skinning_system, update_global_transform_system,
        update_global_transform_with_parent_system,
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
    let test = StressTest::NormalTangentTest;
    let models = init(&mut engine, &test);
    let timer = Default::default();

    let mut tick_props = TickProps {
        engine,
        models,
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
    /// Load a scene with thousands of objects to test culling
    CullingStressTest,
    /// Khronos provided scene to test Image Based Lighting
    IBLTest,
    /// Khronos provided scene to test Normals and Tangents
    NormalTangentTest,
}

fn init(engine: &mut Engine, test: &StressTest) -> HashMap<String, World> {
    let render_context = &mut engine.render_context;
    let vulkan_context = &mut engine.vulkan_context;
    let world = &mut engine.world;

    match test {
        StressTest::ManyCubes => {
            let glb_buffers: Vec<&[u8]> = vec![include_bytes!("../../../test_assets/cube.glb")];
            let models =
                asset_importer::load_models_from_glb(&glb_buffers, vulkan_context, render_context)
                    .unwrap();

            let resolution = 34; // 42,875 cubes

            setup_cubes(world, resolution, &models);

            models
        }
        StressTest::CullingStressTest => {
            let glb_buffers: Vec<&[u8]> = vec![include_bytes!(
                "../../../test_assets/culling_stress_test.glb"
            )];
            let models =
                asset_importer::load_models_from_glb(&glb_buffers, vulkan_context, render_context)
                    .unwrap();

            add_model_to_world("Asteroid and Debris", &models, world, None).unwrap();
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
                let e = add_model_to_world("Damaged Helmet", &models, world, None)
                    .expect("Could not find cube?");
                let mut t = world.get::<&mut GlobalTransform>(e).unwrap();
                t.0 = Affine3A::from_axis_angle(Vec3::X, std::f32::consts::FRAC_PI_2);
            }
            models
        }
        StressTest::ManyVertices => {
            create_mesh(render_context, world);
            Default::default()
        }
        StressTest::Sponza => {
            let file = std::fs::read("test_assets/sponza.glb").unwrap();
            let glb_buffers: Vec<&[u8]> = vec![&file];
            let models =
                asset_importer::load_models_from_glb(&glb_buffers, vulkan_context, render_context)
                    .unwrap();
            for name in models.keys() {
                add_model_to_world(name, &models, world, None);
            }
            models
        }
        StressTest::IBLTest => {
            #[cfg(target_os = "android")]
            let glb_buffers: Vec<&[u8]> =
                vec![include_bytes!("../../../test_assets/ibl_test_squished.glb")];

            #[cfg(not(target_os = "android"))]
            let glb_buffers: Vec<&[u8]> = vec![include_bytes!("../../../test_assets/ibl_test.glb")];

            let models =
                asset_importer::load_models_from_glb(&glb_buffers, vulkan_context, render_context)
                    .unwrap();
            for name in models.keys() {
                add_model_to_world(name, &models, world, None);
            }
            models
        }
        StressTest::NormalTangentTest => {
            let glb_buffers: Vec<&[u8]> = vec![include_bytes!(
                "../../../test_assets/normal_tangent_test.glb"
            )];

            let models =
                asset_importer::load_models_from_glb(&glb_buffers, vulkan_context, render_context)
                    .unwrap();
            for name in models.keys() {
                add_model_to_world(name, &models, world, None);
            }

            let scene_data = &mut render_context.scene_data;

            // Disable IBL
            scene_data.params.x = 0.;

            // Add a spotlight pointing at the scene
            scene_data.lights[0] = Light::new_spotlight(
                [0.5, 0., -1.].into(),
                10.,
                5.,
                [1., 1., 1.].into(),
                [-2., 2., 2.].into(),
                0.,
                0.7853892,
            );

            models
        }
    }
}

struct TickProps {
    engine: Engine,
    models: HashMap<String, World>,
    timer: Timer,
    test: StressTest,
}

fn tick(tick_props: &mut TickProps, tick_data: TickData) {
    let engine = &mut tick_props.engine;
    let timer = &mut tick_props.timer;
    let models = &tick_props.models;

    if tick_data.current_state == xr::SessionState::FOCUSED {
        hands_system(engine);
        grabbing_system(engine);
        physics_system(engine);

        match &tick_props.test {
            // StressTest::ManyCubes => rotate_models(world, timer.total_time().as_secs_f32()),
            StressTest::ManyHelmets => model_system(engine, models, timer, "Damaged Helmet"),
            StressTest::ManyVertices => subdivide_mesh_system(engine, timer),
            _ => {}
        }

        debug_system(engine);

        animation_system(engine);
        update_global_transform_system(engine);
        update_global_transform_with_parent_system(engine);
        skinning_system(engine);
    }

    // Rendering!
    rendering_system(engine, tick_data.swapchain_image_index);
}

fn subdivide_mesh_system(engine: &mut Engine, timer: &mut Timer) {
    let world = &mut engine.world;
    let render_context = &mut engine.render_context;

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
    engine: &mut Engine,
    models: &HashMap<String, World>,
    timer: &mut Timer,
    model_name: &str,
) {
    let world = &mut engine.world;
    if timer.tick() {
        add_model_to_world(model_name, models, world, None).expect("Could not find object?");
        rearrange_models(world);
    }

    rotate_models(world, timer.total_time().as_secs_f32());
}

fn rotate_models(world: &mut World, total_time: f32) {
    for (_, global_transform) in world.query_mut::<With<&mut GlobalTransform, &Mesh>>() {
        let (scale, _, translation) = global_transform.to_scale_rotation_translation();
        global_transform.0 = Affine3A::from_scale_rotation_translation(
            scale,
            Quat::from_euler(
                EulerRot::XYZ,
                90.0_f32.to_radians(),
                total_time.sin() * 2.,
                0.,
            ),
            translation,
        );
    }
}

fn rearrange_models(world: &mut World) {
    let query = world.query_mut::<With<&mut GlobalTransform, &Mesh>>();
    let query_iter = query.into_iter();
    let num_models = query_iter.len() as f32;

    let column_size = num_models.sqrt() as usize;
    let scale = 2. / column_size as f32;
    let half_column_size = column_size as f32 / 2.;
    let mut row = 0;
    let mut column = 0;

    for (_, global_transform) in query_iter {
        if column >= column_size {
            column = 0;
            row += 1;
        }

        global_transform.0 = Affine3A::from_scale_rotation_translation(
            Vec3::splat(scale),
            Quat::IDENTITY,
            vec3((column as f32) - half_column_size, (row as f32) - 0.5, -4.0),
        );

        column += 1;
    }

    println!("[HOTHAM_STRESS_TEST] There are now {num_models} models");
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
    let global_transform = GlobalTransform(Affine3A::from_translation(vec3(0., 1., -1.)));

    world.spawn((Visible {}, mesh, global_transform));
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
        render_context
            .resources
            .position_buffer
            .overwrite(&vertices);
    }

    println!(
        "There are now {} vertices and {} indices",
        vertices.len(),
        indices.len()
    );
}

fn vertex(x: f32, y: f32) -> Vec3 {
    [x, y, -1.0].into()
}
