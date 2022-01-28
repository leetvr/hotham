use hotham::components::hand::Handedness;
use hotham::gltf_loader::add_model_to_world;
use hotham::hecs::World;
use hotham::resources::{RenderContext, XrContext};
use hotham::schedule_functions::{begin_frame, end_frame, physics_step};
use hotham::systems::rendering::rendering_system;
use hotham::systems::skinning::skinning_system;
use hotham::systems::{
    animation_system, collision_system, grabbing_system, hands_system,
    update_parent_transform_matrix_system, update_rigid_body_transforms_system,
    update_transform_matrix_system,
};
use hotham::{gltf_loader, App, HothamResult};

use hotham::{
    components::{AnimationController, Hand, Transform},
    rapier3d::prelude::{ActiveCollisionTypes, ActiveEvents, ColliderBuilder, RigidBodyBuilder},
    resources::PhysicsContext,
};

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
pub fn main() {
    println!("[HOTHAM_ASTEROID_ANDROID] MAIN!");
    real_main().unwrap();
}

pub fn real_main() -> HothamResult<()> {
    let (xr_context, vulkan_context) = XrContext::new()?;
    let render_context = RenderContext::new(&vulkan_context, &xr_context)?;
    let mut physics_context = PhysicsContext::default();
    let glb_bufs: Vec<&[u8]> = vec![
        include_bytes!("../../../test_assets/left_hand.glb"),
        include_bytes!("../../../test_assets/right_hand.glb"),
        include_bytes!("../../../test_assets/damaged_helmet.glb"),
    ];
    let models = gltf_loader::load_models_from_glb(
        &glb_bufs,
        &vulkan_context,
        &render_context.descriptor_set_layouts,
    )?;

    let mut world = World::default();

    // Add the damaged helmet
    let helmet = add_model_to_world(
        "Damaged Helmet",
        &models,
        &mut world,
        None,
        &vulkan_context,
        &render_context.descriptor_set_layouts,
    )
    .expect("Could not find Damaged Helmet");

    // Add the helmet model
    let transform = world.get::<&Transform>(helmet).unwrap();
    let position = transform.position();
    drop(transform);

    // Give it a collider and rigid-body
    let collider = ColliderBuilder::ball(0.35)
        .active_collision_types(ActiveCollisionTypes::all())
        .active_events(ActiveEvents::CONTACT_EVENTS | ActiveEvents::INTERSECTION_EVENTS)
        .build();
    let rigid_body = RigidBodyBuilder::new_dynamic().position(position).build();
    let components = physics_context.get_rigid_body_and_collider(helmet, rigid_body, collider);
    world.insert(helmet, components);

    // Add the left hand
    add_hand(
        &models,
        Handedness::Left,
        &mut world,
        &vulkan_context,
        &render_context,
        &mut physics_context,
    );

    // Add the right hand
    add_hand(
        &models,
        Handedness::Right,
        &mut world,
        &vulkan_context,
        &render_context,
        &mut physics_context,
    );

    // let mut resources = Resources::default();
    // resources.insert(xr_context);
    // resources.insert(vulkan_context);
    // resources.insert(render_context);
    // resources.insert(physics_context);
    // resources.insert(0 as usize);
    // let schedule = Schedule::builder()
    //     .add_thread_local_fn(begin_frame)
    //     .add_system(hands_system())
    //     .add_system(collision_system())
    //     .add_system(grabbing_system())
    //     .add_thread_local_fn(physics_step)
    //     .add_system(update_rigid_body_transforms_system())
    //     .add_system(animation_system())
    //     .add_system(update_transform_matrix_system())
    //     .add_system(update_parent_transform_matrix_system())
    //     .add_system(skinning_system())
    //     .add_system(rendering_system())
    //     .add_thread_local_fn(end_frame)
    //     .build();
    // println!("[HOTHAM_INIT] DONE! INIT COMPLETE!");

    // let mut app = App::new(world, resources, schedule)?;
    // app.run()?;
    Ok(())
}

fn add_hand(
    models: &std::collections::HashMap<String, World>,
    handedness: Handedness,
    world: &mut World,
    vulkan_context: &hotham::resources::vulkan_context::VulkanContext,
    render_context: &RenderContext,
    physics_context: &mut PhysicsContext,
) {
    let model_name = match handedness {
        Handedness::Left => "Left Hand",
        Handedness::Right => "Right Hand",
    };
    let left_hand = add_model_to_world(
        model_name,
        models,
        world,
        None,
        vulkan_context,
        &render_context.descriptor_set_layouts,
    )
    .unwrap();
    {
        // Add a hand component
        world.insert_one(left_hand, Hand::left());

        // Modify the animation controller
        let animation_controller = world
            .get_mut::<&mut AnimationController>(left_hand)
            .unwrap();
        animation_controller.blend_from = 0;
        animation_controller.blend_to = 1;

        // Give it a collider and rigid-body
        let collider = ColliderBuilder::capsule_y(0.05, 0.02)
            .sensor(true)
            .active_collision_types(ActiveCollisionTypes::all())
            .active_events(ActiveEvents::CONTACT_EVENTS | ActiveEvents::INTERSECTION_EVENTS)
            .build();
        let rigid_body = RigidBodyBuilder::new_kinematic_position_based().build();
        let components =
            physics_context.get_rigid_body_and_collider(left_hand, rigid_body, collider);
        world.insert(left_hand, components);
    }
}
