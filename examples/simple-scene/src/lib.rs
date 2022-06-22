use hotham::{
    asset_importer::{self, add_model_to_world},
    components::{hand::Handedness, Transform},
    hecs::World,
    rapier3d::prelude::{ActiveCollisionTypes, ActiveEvents, ColliderBuilder, RigidBodyBuilder},
    resources::{vulkan_context::VulkanContext, PhysicsContext, RenderContext},
    schedule_functions::{
        begin_frame, begin_pbr_renderpass, end_frame, end_pbr_renderpass, physics_step,
    },
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

    while let Ok((previous_state, current_state)) = engine.update() {
        tick(
            previous_state,
            current_state,
            &mut engine,
            &mut world,
            &mut queries,
        );
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
        include_bytes!("../../../test_assets/damaged_helmet.glb"),
    ];
    let models =
        asset_importer::load_models_from_glb(&glb_buffers, vulkan_context, render_context)?;
    add_helmet(
        &models,
        &mut world,
        vulkan_context,
        render_context,
        physics_context,
    );
    add_hand(
        &models,
        Handedness::Left,
        &mut world,
        vulkan_context,
        render_context,
        physics_context,
    );
    add_hand(
        &models,
        Handedness::Right,
        &mut world,
        vulkan_context,
        render_context,
        physics_context,
    );

    Ok(world)
}

fn add_helmet(
    models: &std::collections::HashMap<String, World>,
    world: &mut World,
    vulkan_context: &mut VulkanContext,
    render_context: &mut RenderContext,
    physics_context: &mut PhysicsContext,
) {
    let helmet = add_model_to_world(
        "Damaged Helmet",
        models,
        world,
        None,
        vulkan_context,
        render_context,
    )
    .expect("Could not find Damaged Helmet");
    let transform = world.get::<Transform>(helmet).unwrap();
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
    _previous_state: xr::SessionState,
    current_state: xr::SessionState,
    engine: &mut Engine,
    world: &mut World,
    queries: &mut Queries,
) {
    // If we're not in a session, don't run the frame loop.
    match current_state {
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
