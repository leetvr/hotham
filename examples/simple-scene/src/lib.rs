use hotham::{
    asset_importer::{self, add_model_to_world},
    components::{hand::Handedness, Transform},
    hecs::World,
    rapier3d::prelude::{ActiveCollisionTypes, ActiveEvents, ColliderBuilder, RigidBodyBuilder},
    resources::{PhysicsContext, RenderContext, XrContext},
    schedule_functions::{begin_frame, end_frame, physics_step},
    systems::{
        animation_system, collision_system, grabbing_system, hands::add_hand, hands_system,
        rendering::rendering_system, skinning::skinning_system,
        update_parent_transform_matrix_system, update_rigid_body_transforms_system,
        update_transform_matrix_system, Queries,
    },
    xr, Engine, HothamResult,
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

    while let Ok(xr_state) = engine.update() {
        tick(xr_state, &mut engine, &mut world, &mut queries, &mut state);
    }

    Ok(())
}

fn init(engine: &mut Engine) -> Result<World, hotham::HothamError> {
    let render_context = &mut engine.render_context;
    let vulkan_context = &mut engine.vulkan_context;
    let physics_context = &mut engine.physics_context;
    let mut world = World::default();

    let glb_buffers: Vec<&[u8]> = vec![
        include_bytes!("../../../test_assets/left_hand.glb"),
        include_bytes!("../../../test_assets/right_hand.glb"),
        include_bytes!("../../../test_assets/damaged_helmet_squished.glb"),
    ];
    let models =
        asset_importer::load_models_from_glb(&glb_buffers, vulkan_context, render_context)?;
    add_helmet(&models, &mut world, physics_context);
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
    let mut transform = world.get_mut::<Transform>(helmet).unwrap();
    transform.translation.z = -1.;
    transform.translation.y = 0.6;
    transform.scale = [0.5, 0.5, 0.5].into();
    let position = transform.position();
    drop(transform);
    let collider = ColliderBuilder::ball(0.35)
        .active_collision_types(ActiveCollisionTypes::all())
        .active_events(ActiveEvents::CONTACT_EVENTS | ActiveEvents::INTERSECTION_EVENTS)
        .build();
    let rigid_body = RigidBodyBuilder::new_dynamic().position(position).build();
    let components = physics_context.get_rigid_body_and_collider(helmet, rigid_body, collider);
    world.insert(helmet, components).unwrap();
}

fn tick(
    xr_state: (xr::SessionState, xr::SessionState),
    engine: &mut Engine,
    world: &mut World,
    queries: &mut Queries,
    state: &mut State,
) {
    let current_state = xr_state.1;
    // If we're not in a session, don't run the frame loop.
    match xr_state.1 {
        xr::SessionState::IDLE | xr::SessionState::EXITING | xr::SessionState::STOPPING => return,
        _ => {}
    }

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
        animation_system(&mut queries.animation_query, world);
        update_transform_matrix_system(&mut queries.update_transform_matrix_query, world);
        update_parent_transform_matrix_system(
            &mut queries.parent_query,
            &mut queries.roots_query,
            world,
        );
        debug_system(xr_context, render_context, state);
        skinning_system(&mut queries.skins_query, world, render_context);
    }

    if current_state == xr::SessionState::FOCUSED || current_state == xr::SessionState::VISIBLE {
        rendering_system(
            &mut queries.rendering_query,
            world,
            vulkan_context,
            xr_context.frame_index,
            render_context,
        );
    }

    end_frame(xr_context, vulkan_context, render_context);
}

#[derive(Clone, Debug, Default)]
struct State {
    debug_state: DebugState,
}

#[derive(Clone, Debug, Default)]
struct DebugState {
    button_pressed_last_frame: bool,
}

fn debug_system(xr_context: &mut XrContext, render_context: &mut RenderContext, state: &mut State) {
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
