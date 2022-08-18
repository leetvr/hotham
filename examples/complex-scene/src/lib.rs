pub mod navigation;

use hotham::{
    asset_importer::{self, add_model_to_world},
    components::{hand::Handedness, Hologram, LocalTransform},
    contexts::PhysicsContext,
    glam::{self, Mat4, Quat},
    hecs::World,
    rapier3d::prelude::{
        ActiveCollisionTypes, ActiveEvents, ColliderBuilder, RigidBodyBuilder, RigidBodyType,
    },
    systems::{
        animation_system, debug::debug_system, grabbing_system, hands::add_hand, hands_system,
        physics_system, rendering::rendering_system, skinning::skinning_system,
        update_global_transform_system, update_global_transform_with_parent_system,
    },
    xr, Engine, HothamResult, TickData,
};
use navigation::navigation_system;

#[derive(Clone, Debug, Default)]
/// The state is used for manipulating the stage transform
pub struct State {
    global_from_left_grip: Option<glam::Affine3A>,
    global_from_right_grip: Option<glam::Affine3A>,
    scale: Option<f32>,
}

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
pub fn main() {
    println!("[HOTHAM_COMPLEX_SCENE] MAIN!");
    real_main().expect("Error running app!");
    println!("[HOTHAM_COMPLEX_SCENE] FINISHED! Goodbye!");
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

fn tick(tick_data: TickData, engine: &mut Engine, state: &mut State) {
    if tick_data.current_state == xr::SessionState::FOCUSED {
        hands_system(engine);
        grabbing_system(engine);
        physics_system(engine);
        animation_system(engine);
        navigation_system(engine, state);
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
    let physics_context = &mut engine.physics_context;
    let world = &mut engine.world;

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

    let models = asset_importer::load_models_from_glb(
        &glb_buffers,
        vulkan_context,
        render_context,
        physics_context,
    )?;
    add_helmet(&models, world, physics_context);
    add_hand(&models, Handedness::Left, world, physics_context);
    add_hand(&models, Handedness::Right, world, physics_context);
    add_quadric(
        &models,
        world,
        physics_context,
        &LocalTransform {
            translation: [1.0, 1.4, -1.5].into(),
            rotation: Quat::IDENTITY,
            scale: [0.5, 0.5, 0.5].into(),
        },
        0.5,
        Hologram {
            surface_q_in_local: Mat4::from_diagonal([1.0, 1.0, 1.0, -1.0].into()),
            bounds_q_in_local: Mat4::from_diagonal([0.0, 0.0, 0.0, 0.0].into()),
            uv_from_local: Mat4::IDENTITY,
        },
    );
    add_quadric(
        &models,
        world,
        physics_context,
        &LocalTransform {
            translation: [-1.0, 1.4, -1.5].into(),
            rotation: Quat::IDENTITY,
            scale: [0.5, 0.5, 0.5].into(),
        },
        0.5,
        Hologram {
            surface_q_in_local: Mat4::from_diagonal([1.0, 1.0, 0.0, -1.0].into()),
            bounds_q_in_local: Mat4::from_diagonal([0.0, 0.0, 1.0, -1.0].into()),
            uv_from_local: Mat4::IDENTITY,
        },
    );

    Ok(())
}

fn add_helmet(
    models: &std::collections::HashMap<String, World>,
    world: &mut World,
    physics_context: &mut PhysicsContext,
) {
    let helmet = add_model_to_world("Damaged Helmet", models, world, physics_context, None)
        .expect("Could not find Damaged Helmet");
    let mut local_transform = world.get::<&mut LocalTransform>(helmet).unwrap();
    local_transform.translation.z = -1.;
    local_transform.translation.y = 1.4;
    local_transform.scale = [0.5, 0.5, 0.5].into();
    let position = local_transform.to_isometry();
    drop(local_transform);
    let collider = ColliderBuilder::ball(0.35)
        .active_collision_types(ActiveCollisionTypes::all())
        .active_events(ActiveEvents::COLLISION_EVENTS)
        .build();
    let rigid_body = RigidBodyBuilder::new(RigidBodyType::Dynamic)
        .position(position)
        .build();
    let components = physics_context.create_rigid_body_and_collider(helmet, rigid_body, collider);
    world.insert(helmet, components).unwrap();
}

fn add_quadric(
    models: &std::collections::HashMap<String, World>,
    world: &mut World,
    physics_context: &mut PhysicsContext,
    local_transform: &LocalTransform,
    ball_radius: f32,
    hologram: Hologram,
) {
    let entity = add_model_to_world("Sphere", models, world, physics_context, None)
        .expect("Could not find Sphere");
    *world.get::<&mut LocalTransform>(entity).unwrap() = *local_transform;
    let position = local_transform.to_isometry();
    let collider = ColliderBuilder::ball(ball_radius)
        .active_collision_types(ActiveCollisionTypes::all())
        .active_events(ActiveEvents::COLLISION_EVENTS)
        .build();
    let rigid_body = RigidBodyBuilder::new(RigidBodyType::Dynamic)
        .position(position)
        .build();
    let components = physics_context.create_rigid_body_and_collider(entity, rigid_body, collider);
    world
        .insert(entity, (components.0, components.1, hologram))
        .unwrap();
}
