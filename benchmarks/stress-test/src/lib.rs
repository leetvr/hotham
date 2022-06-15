use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use hotham::{
    components::Transform,
    gltf_loader::{self, add_model_to_world},
    hecs::{With, World},
    nalgebra::Vector3,
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
    xr, Engine, HothamResult,
};

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
pub fn main() {
    println!("[HOTHAM_STRESS_TEST] MAIN!");
    real_main().expect("Error running app!");
    println!("[HOTHAM_STRESS_TEST] FINISHED! Goodbye!");
}

pub fn real_main() -> HothamResult<()> {
    let mut engine = Engine::new();
    let (world, models) = init(&mut engine)?;
    let queries = Default::default();
    let timer = Default::default();

    let mut tick_props = TickProps {
        engine,
        world,
        models,
        queries,
        timer,
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

fn init(engine: &mut Engine) -> Result<(World, HashMap<String, World>), hotham::HothamError> {
    let render_context = &mut engine.render_context;
    let vulkan_context = &mut engine.vulkan_context;
    let mut world = World::default();

    let glb_buffers: Vec<&[u8]> = vec![include_bytes!("../../../test_assets/cube.glb")];
    let models = gltf_loader::load_models_from_glb(
        &glb_buffers,
        vulkan_context,
        &render_context.descriptor_set_layouts,
    )?;

    add_cube(&models, &mut world, vulkan_context, render_context);

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
        cube_system(world, models, vulkan_context, render_context, timer);
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
