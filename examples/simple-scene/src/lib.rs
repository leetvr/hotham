use hotham::{
    asset_importer::{self, add_model_to_world},
    components::{hand::Handedness, LocalTransform},
    contexts::PhysicsContext,
    hecs::World,
    rapier3d::prelude::{
        ActiveCollisionTypes, ActiveEvents, ColliderBuilder, RigidBodyBuilder, RigidBodyType,
    },
    systems::{
        animation_system, collision_system, grabbing_system, hands::add_hand, hands_system,
        physics_system, rendering::rendering_system, skinning::skinning_system,
        update_global_transform_system, update_global_transform_with_parent_system,
        update_local_transform_with_rigid_body_system,
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
        collision_system(engine);
        update_local_transform_with_rigid_body_system(engine);
        animation_system(engine);
        update_global_transform_system(engine);
        update_global_transform_with_parent_system(engine);
        skinning_system(engine);
    }

    rendering_system(engine, tick_data.swapchain_image_index);
}

fn init(engine: &mut Engine) -> Result<(), hotham::HothamError> {
    let render_context = &mut engine.render_context;

    let vulkan_context = &mut engine.vulkan_context;
    let physics_context = &mut engine.physics_context;
    let world = &mut engine.world;

    let mut glb_buffers: Vec<&[u8]> = vec![
        include_bytes!("../../../test_assets/left_hand.glb"),
        include_bytes!("../../../test_assets/right_hand.glb"),
    ];

    #[cfg(target_os = "android")]
    glb_buffers.push(include_bytes!(
        "../../../test_assets/damaged_helmet_squished.glb"
    ));

    #[cfg(not(target_os = "android"))]
    glb_buffers.push(include_bytes!("../../../test_assets/damaged_helmet.glb"));

    let models =
        asset_importer::load_models_from_glb(&glb_buffers, vulkan_context, render_context)?;
    add_helmet(&models, world, physics_context);
    add_hand(&models, Handedness::Left, world, physics_context);
    add_hand(&models, Handedness::Right, world, physics_context);

    Ok(())
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
