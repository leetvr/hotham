use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use hotham::{
    asset_importer::{self, add_model_to_world},
    components::{mesh::MeshUBO, Material, Mesh, Primitive, Transform, TransformMatrix, Visible},
    hecs::{With, World},
    nalgebra::Vector3,
    rendering::{buffer::Buffer, texture::Texture, vertex::Vertex},
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
    ManyVertices,
    Sponza,
}

fn init(
    engine: &mut Engine,
    test: &StressTest,
) -> Result<(World, HashMap<String, World>), hotham::HothamError> {
    let render_context = &mut engine.render_context;
    let vulkan_context = &mut engine.vulkan_context;
    let mut world = World::default();

    let models = match test {
        StressTest::ManyCubes => {
            let glb_buffers: Vec<&[u8]> = vec![include_bytes!("../../../test_assets/cube.glb")];
            let models = asset_importer::load_models_from_glb(
                &glb_buffers,
                vulkan_context,
                &render_context.descriptor_set_layouts,
            )?;

            add_cube(&models, &mut world, vulkan_context, render_context);
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
                &render_context.descriptor_set_layouts,
            )?;
            for name in models.keys() {
                add_model_to_world(
                    name,
                    &models,
                    &mut world,
                    None,
                    vulkan_context,
                    &render_context.descriptor_set_layouts,
                );
            }
            models
        }
    };

    Ok((world, models))
}

fn add_cube(
    models: &std::collections::HashMap<String, World>,
    world: &mut World,
    vulkan_context: &VulkanContext,
    render_context: &RenderContext,
) {
    let cube = add_model_to_world(
        "Cube",
        models,
        world,
        None,
        vulkan_context,
        &render_context.descriptor_set_layouts,
    )
    .expect("Could not find cube?");
    world.insert_one(cube, Cube {}).unwrap();
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
            StressTest::ManyCubes => {
                cube_system(world, models, vulkan_context, render_context, timer)
            }
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
        skinning_system(&mut queries.joints_query, &mut queries.meshes_query, world);
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
    if !timer.tick() {
        return;
    }

    let mesh = world.query_mut::<&mut Mesh>().into_iter().next().unwrap().1;
    let step = timer.total_time().as_secs() * 10;
    let (vertices, indices) = generate_vertices(step as _);
    let primitive = &mut mesh.primitives[0];
    primitive.indices_count = indices.len() as _;
    primitive
        .index_buffer
        .update(vulkan_context, &indices)
        .unwrap();
    primitive
        .vertex_buffer
        .update(vulkan_context, &vertices)
        .unwrap();

    println!(
        "There are now {} vertices and {} indices",
        vertices.len(),
        indices.len()
    );
}

struct Cube;

fn cube_system(
    world: &mut World,
    models: &HashMap<String, World>,
    vulkan_context: &VulkanContext,
    render_context: &RenderContext,
    timer: &mut Timer,
) {
    if timer.tick() {
        add_cube(models, world, vulkan_context, render_context);
        rearrange_cubes(world);
    }

    rotate_cubes(world, timer.total_time().as_secs_f32());
}

fn rotate_cubes(world: &mut World, total_time: f32) {
    for (_, transform) in world.query_mut::<With<Cube, &mut Transform>>() {
        transform.rotation =
            hotham::nalgebra::Rotation::from_axis_angle(&Vector3::y_axis(), total_time.sin() * 2.)
                .into();
    }
}

fn rearrange_cubes(world: &mut World) {
    let query = world.query_mut::<With<Cube, &mut Transform>>();
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
    let mesh_layout = render_context.descriptor_set_layouts.mesh_layout;
    let mesh_ubo = MeshUBO::default();
    let num_vertices = 1_000_000;
    let mut vertices = Vec::with_capacity(num_vertices);
    let mut indices = Vec::with_capacity(num_vertices);

    for _ in 0..num_vertices {
        vertices.push(Vertex::default());
        indices.push(0);
    }

    let vertex_buffer = Buffer::new(
        vulkan_context,
        &vertices,
        vk::BufferUsageFlags::VERTEX_BUFFER,
    )
    .unwrap();
    let index_buffer =
        Buffer::new(vulkan_context, &indices, vk::BufferUsageFlags::INDEX_BUFFER).unwrap();

    let texture_descriptor_set = vulkan_context.create_textures_descriptor_sets(
        render_context.descriptor_set_layouts.textures_layout,
        "Empty Material",
        &[
            &Texture::empty(vulkan_context)?,
            &Texture::empty(vulkan_context)?,
            &Texture::empty(vulkan_context)?,
            &Texture::empty(vulkan_context)?,
            &Texture::empty(vulkan_context)?,
        ],
    )?[0];
    let material = Material::default();

    let primitives = vec![Primitive {
        index_buffer,
        vertex_buffer,
        indices_count: 0,
        material,
        texture_descriptor_set,
    }];

    // Vomit
    let descriptor_sets = [vulkan_context
        .create_mesh_descriptor_sets(mesh_layout, "Dynamic Mesh")
        .unwrap()[0]];

    let ubo_buffer = Buffer::new(
        vulkan_context,
        &[mesh_ubo],
        vk::BufferUsageFlags::UNIFORM_BUFFER,
    )
    .unwrap();

    vulkan_context.update_buffer_descriptor_set(
        &ubo_buffer,
        descriptor_sets[0],
        0,
        vk::DescriptorType::UNIFORM_BUFFER,
    );
    let mesh = Mesh {
        descriptor_sets,
        ubo_data: mesh_ubo,
        ubo_buffer,
        primitives,
    };

    let transform = Transform {
        translation: [0., 1., -1.].into(),
        ..Default::default()
    };

    world.spawn((Visible {}, mesh, transform, TransformMatrix::default()));
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
