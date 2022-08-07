use hotham::{
    asset_importer::{self, add_model_to_world},
    components::{hand::Handedness, LocalTransform},
    hecs::World,
    rapier3d::prelude::{
        ActiveCollisionTypes, ActiveEvents, ColliderBuilder, RigidBodyBuilder, RigidBodyType,
    },
    resources::{PhysicsContext, RenderContext, XrContext},
    schedule_functions::physics_step,
    systems::{
        animation_system, collision_system, grabbing_system, hands::add_hand, hands_system,
        rendering::rendering_system, skinning::skinning_system, update_global_transform_system,
        update_global_transform_with_parent_system, update_local_transform_with_rigid_body_system,
        Queries,
    },
    xr, Engine, HothamResult, TickData,
};

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
pub fn main() {
    println!("[HOTHAM_SIMPLE_SCENE] MAIN!");
    real_main().expect("Error running app!");
    println!("[HOTHAM_SIMPLE_SCENE] FINISHED! Goodbye!");
}

pub fn real_main() -> HothamResult<()> {
    let mut engine = Engine::new();
    let mut world = init(&mut engine)?;
    let mut queries = Default::default();
    let mut state = Default::default();

    while let Ok(tick_data) = engine.update() {
        tick(tick_data, &mut engine, &mut world, &mut queries, &mut state);
        engine.finish()?;
    }

    Ok(())
}

fn init(engine: &mut Engine) -> Result<World, hotham::HothamError> {
    let render_context = &mut engine.render_context;

    let vulkan_context = &mut engine.vulkan_context;
    let physics_context = &mut engine.physics_context;
    let mut world = World::default();

    let mut glb_buffers: Vec<&[u8]> = vec![
        include_bytes!("../../../test_assets/left_hand.glb"),
        include_bytes!("../../../test_assets/right_hand.glb"),
        include_bytes!("../../../test_assets/sphere.glb"),
    ];

    #[cfg(target_os = "android")]
    glb_buffers.push(include_bytes!(
        "../../../test_assets/damaged_helmet_squished.glb"
    ));

    #[cfg(not(target_os = "android"))]
    glb_buffers.push(include_bytes!("../../../test_assets/damaged_helmet.glb"));

    let models =
        asset_importer::load_models_from_glb(&glb_buffers, vulkan_context, render_context)?;
    add_helmet(&models, &mut world, physics_context);
    add_sphere(&models, &mut world, physics_context);
    add_hand(&models, Handedness::Left, &mut world, physics_context);
    add_hand(&models, Handedness::Right, &mut world, physics_context);

    Ok(world)
}

fn add_helmet(
    models: &std::collections::HashMap<String, World>,
    world: &mut World,
    physics_context: &mut PhysicsContext,
) {
    let helmet = add_model_to_world("Damaged Helmet", models, world, None)
        .expect("Could not find Damaged Helmet");
    let mut local_transform = world.get_mut::<LocalTransform>(helmet).unwrap();
    local_transform.translation.z = -1.;
    local_transform.translation.y = 1.4;
    local_transform.scale = [0.5, 0.5, 0.5].into();
    let position = local_transform.position();
    drop(local_transform);
    let collider = ColliderBuilder::ball(0.35)
        .active_collision_types(ActiveCollisionTypes::all())
        .active_events(ActiveEvents::COLLISION_EVENTS)
        .build();
    let rigid_body = RigidBodyBuilder::new(RigidBodyType::Dynamic)
        .position(position)
        .build();
    let components = physics_context.get_rigid_body_and_collider(helmet, rigid_body, collider);
    world.insert(helmet, components).unwrap();
}

fn add_sphere(
    models: &std::collections::HashMap<String, World>,
    world: &mut World,
    physics_context: &mut PhysicsContext,
) {
    let entity = add_model_to_world("Sphere", models, world, None).expect("Could not find Sphere");
    let mut local_transform = world.get_mut::<LocalTransform>(entity).unwrap();
    local_transform.translation.x = 0.5;
    local_transform.translation.z = -1.0;
    local_transform.translation.y = 1.4;
    local_transform.scale = [0.5, 0.5, 0.5].into();
    let position = local_transform.position();
    drop(local_transform);
    let collider = ColliderBuilder::ball(0.5)
        .active_collision_types(ActiveCollisionTypes::all())
        .active_events(ActiveEvents::COLLISION_EVENTS)
        .build();
    let rigid_body = RigidBodyBuilder::new(RigidBodyType::Dynamic)
        .position(position)
        .build();
    let components = physics_context.get_rigid_body_and_collider(entity, rigid_body, collider);
    world.insert(entity, components).unwrap();
}

fn tick(
    tick_data: TickData,
    engine: &mut Engine,
    world: &mut World,
    queries: &mut Queries,
    state: &mut State,
) {
    let xr_context = &mut engine.xr_context;
    let vulkan_context = &engine.vulkan_context;
    let render_context = &mut engine.render_context;
    let physics_context = &mut engine.physics_context;

    if tick_data.current_state == xr::SessionState::FOCUSED {
        hands_system(&mut queries.hands_query, world, xr_context, physics_context);
        grabbing_system(&mut queries.grabbing_query, world, physics_context);
        physics_step(physics_context);
        collision_system(&mut queries.collision_query, world, physics_context);
        update_local_transform_with_rigid_body_system(
            &mut queries.update_rigid_body_transforms_query,
            world,
            physics_context,
        );
        animation_system(&mut queries.animation_query, world);
        update_global_transform_system(&mut queries.update_global_transform_query, world);
        update_global_transform_with_parent_system(
            &mut queries.parent_query,
            &mut queries.roots_query,
            world,
        );

        debug_system(xr_context, render_context, state);

        skinning_system(&mut queries.skins_query, world, render_context);
    }

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

#[derive(Clone, Debug, Default)]
struct State {
    debug_state: DebugState,
}

#[derive(Clone, Debug, Default)]
struct DebugState {
    button_pressed_last_frame: bool,
}

#[allow(unused)]
fn debug_system(xr_context: &mut XrContext, render_context: &mut RenderContext, state: &mut State) {
    #[cfg(not(target_os = "android"))]
    return;

    let input = &xr_context.input;
    let pressed = xr::ActionInput::get(
        &input.y_button_action,
        &xr_context.session,
        xr_context.input.left_hand_subaction_path,
    )
    .unwrap()
    .current_state;

    if state.debug_state.button_pressed_last_frame && pressed {
        return;
    }

    if pressed {
        let debug_data = &mut render_context.scene_data.debug_data;
        debug_data.x = ((debug_data.x as usize + 1) % 6) as f32;
        println!("debug_data.x is now {}", debug_data.x);
    }

    state.debug_state.button_pressed_last_frame = pressed;
}
