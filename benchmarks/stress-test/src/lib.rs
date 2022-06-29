use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use hotham::{
    asset_importer::{self, add_model_to_world},
    components::{Mesh, Transform, TransformMatrix, Visible},
    hecs::{With, World},
    nalgebra::Vector3,
    rendering::{buffer::Buffer, material::Material, texture::Texture, vertex::Vertex},
    resources::{vulkan_context::VulkanContext, RenderContext},
    schedule_functions::{
        begin_frame, begin_pbr_renderpass, end_frame, end_pbr_renderpass, physics_step,
    },
    systems::{
        animation_system, collision_system, grabbing_system, hands_system,
        rendering::rendering_system, skinning::skinning_system,
        update_parent_transform_matrix_system, update_rigid_body_transforms_system,
        update_transform_matrix_system, Queries,
    },
    vk, xr, Engine, HothamResult,
};

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
pub fn main() {
    println!("[HOTHAM_STRESS_TEST] MAIN!");
    real_main().expect("Error running app!");
    println!("[HOTHAM_STRESS_TEST] FINISHED! Goodbye!");
}

pub fn real_main() -> HothamResult<()> {
    let mut engine = Engine::new();
    let test = StressTest::ManyCubes;
    let (world, models) = init(&mut engine, &test)?;
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

    while let Ok((previous_state, current_state)) = tick_props.engine.update() {
        tick(&mut tick_props, previous_state, current_state);
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
enum StressTest {
    ManyCubes,
    ManyHelmets,
    ManyVertices,
    Sponza,
}

fn init(
    engine: &mut Engine,
    test: &StressTest,
) -> Result<(World, HashMap<String, World>), hotham::HothamError> {
    let mut render_context = &mut engine.render_context;
    let vulkan_context = &mut engine.vulkan_context;
    let mut world = World::default();

    let models = match test {
        StressTest::ManyCubes => {
            let glb_buffers: Vec<&[u8]> = vec![include_bytes!("../../../test_assets/cube.glb")];
            let models = asset_importer::load_models_from_glb(
                &glb_buffers,
                vulkan_context,
                &mut render_context,
            )?;

            add_model_to_world("Cube", &models, &mut world, None).expect("Could not find cube?");
            models
        }
        StressTest::ManyHelmets => {
            let glb_buffers: Vec<&[u8]> =
                vec![include_bytes!("../../../test_assets/damaged_helmet.glb")];
            let models = asset_importer::load_models_from_glb(
                &glb_buffers,
                vulkan_context,
                &mut render_context,
            )?;

            add_model_to_world("Damaged Helmet", &models, &mut world, None)
                .expect("Could not find cube?");
            models
        }
        StressTest::ManyVertices => {
            create_mesh(render_context, vulkan_context, &mut world)?;
            Default::default()
        }
        StressTest::Sponza => {
            let file = std::fs::read("test_assets/sponza.glb").unwrap();
            let glb_buffers: Vec<&[u8]> = vec![&file];
            let models = asset_importer::load_models_from_glb(
                &glb_buffers,
                vulkan_context,
                &mut render_context,
            )?;
            for name in models.keys() {
                add_model_to_world(name, &models, &mut world, None);
            }
            models
        }
    };

    Ok((world, models))
}

struct TickProps<'a> {
    engine: Engine,
    world: World,
    models: HashMap<String, World>,
    queries: Queries<'a>,
    timer: Timer,
    test: StressTest,
}

fn tick(
    tick_props: &mut TickProps,
    _previous_state: xr::SessionState,
    current_state: xr::SessionState,
) {
    // If we're not in a session, don't run the frame loop.
    match current_state {
        xr::SessionState::IDLE | xr::SessionState::EXITING | xr::SessionState::STOPPING => return,
        _ => {}
    }

    let engine = &mut tick_props.engine;
    let world = &mut tick_props.world;
    let queries = &mut tick_props.queries;
    let timer = &mut tick_props.timer;
    let models = &tick_props.models;

    let xr_context = &mut engine.xr_context;
    let vulkan_context = &engine.vulkan_context;
    let render_context = &mut engine.render_context;
    let physics_context = &mut engine.physics_context;

    begin_frame(xr_context, vulkan_context, render_context);

    if current_state == xr::SessionState::FOCUSED {
        hands_system(&mut queries.hands_query, world, xr_context, physics_context);
        grabbing_system(&mut queries.grabbing_query, world, physics_context);
        physics_step(physics_context);
        collision_system(&mut queries.collision_query, world, physics_context);
        update_rigid_body_transforms_system(
            &mut queries.update_rigid_body_transforms_query,
            world,
            physics_context,
        );

        match tick_props.test {
            StressTest::ManyCubes => model_system(world, models, timer, "Cube"),
            StressTest::ManyHelmets => model_system(world, models, timer, "Damaged Helmet"),
            StressTest::ManyVertices => subdivide_mesh_system(world, vulkan_context, timer),
            StressTest::Sponza => {}
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

    if current_state == xr::SessionState::FOCUSED || current_state == xr::SessionState::VISIBLE {
        begin_pbr_renderpass(xr_context, vulkan_context, render_context);
        rendering_system(
            &mut queries.rendering_query,
            world,
            vulkan_context,
            xr_context.frame_index,
            render_context,
        );
        end_pbr_renderpass(xr_context, vulkan_context, render_context);
    }

    end_frame(xr_context, vulkan_context, render_context);
}

fn subdivide_mesh_system(world: &mut World, vulkan_context: &VulkanContext, timer: &mut Timer) {
    todo!()
    // if !timer.tick() {
    //     return;
    // }

    // let mesh = world.query_mut::<&mut Mesh>().into_iter().next().unwrap().1;
    // let step = timer.total_time().as_secs() * 10;
    // let (vertices, indices) = generate_vertices(step as _);
    // let primitive = &mut mesh.primitives[0];
    // primitive.indices_count = indices.len() as _;
    // primitive
    //     .index_buffer
    //     .update(vulkan_context, &indices)
    //     .unwrap();
    // primitive
    //     .vertex_buffer
    //     .update(vulkan_context, &vertices)
    //     .unwrap();

    // println!(
    //     "There are now {} vertices and {} indices",
    //     vertices.len(),
    //     indices.len()
    // );
}

struct Cube;

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
            hotham::nalgebra::Rotation::from_axis_angle(&Vector3::y_axis(), total_time.sin() * 2.)
                .into();
    }
}

fn rearrange_models(world: &mut World) {
    let query = world.query_mut::<With<Mesh, &mut Transform>>();
    let query_iter = query.into_iter();
    let num_cubes = query_iter.len() as f32;
    let slice = std::f32::consts::TAU / num_cubes;
    let scale = 1. / num_cubes;

    for (n, (_, transform)) in query_iter.enumerate() {
        let radius = slice * (n as f32);
        let rotation = hotham::nalgebra::Rotation::from_axis_angle(&Vector3::y_axis(), radius);
        let distance = [0., 0., -2.].into();
        let translation = rotation.transform_vector(&distance);

        transform.translation = translation;
        transform.scale = Vector3::repeat(scale);
    }

    println!("[HOTHAM_STRESS_TEST] There are now {} cubes", num_cubes);
}

// This is.. disgusting.
fn create_mesh(
    render_context: &mut RenderContext,
    vulkan_context: &mut VulkanContext,
    world: &mut World,
) -> Result<(), hotham::HothamError> {
    // let mesh_layout = render_context.descriptor_set_layouts.mesh_layout;
    // let num_vertices = 1_000_000;
    // let mut vertices = Vec::with_capacity(num_vertices);
    // let mut indices = Vec::with_capacity(num_vertices);

    // for _ in 0..num_vertices {
    //     vertices.push(Vertex::default());
    //     indices.push(0);
    // }

    // let vertex_buffer = Buffer::new(
    //     vulkan_context,
    //     &vertices,
    //     vk::BufferUsageFlags::VERTEX_BUFFER,
    // )
    // .unwrap();
    // let index_buffer =
    //     Buffer::new(vulkan_context, &indices, vk::BufferUsageFlags::INDEX_BUFFER).unwrap();

    // let material = Material::default();

    todo!();

    let transform = Transform {
        translation: [0., 1., -1.].into(),
        ..Default::default()
    };

    // world.spawn((Visible {}, mesh, transform, TransformMatrix::default()));
    Ok(())
}

fn generate_vertices(step: usize) -> (Vec<Vertex>, Vec<u32>) {
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

    (vertices, indices)
}

fn vertex(x: f32, y: f32) -> Vertex {
    Vertex {
        position: [x, y, -1.0].into(),
        ..Default::default()
    }
}
