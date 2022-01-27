use hotham::gltf_loader::add_model_to_world;
use hotham::legion::Schedule;
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
    legion::{IntoQuery, Resources, World},
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
    {
        let mut query = <&Transform>::query();
        let transform = query.get(&mut world, helmet).unwrap();
        let position = transform.position();

        let mut helmet_entry = world.entry(helmet).unwrap();
        // Give it a collider and rigid-body
        let collider = ColliderBuilder::ball(0.35)
            .active_collision_types(ActiveCollisionTypes::all())
            .active_events(ActiveEvents::CONTACT_EVENTS | ActiveEvents::INTERSECTION_EVENTS)
            .build();
        let rigid_body = RigidBodyBuilder::new_dynamic().position(position).build();
        let (collider, rigid_body) =
            physics_context.get_rigid_body_and_collider(helmet, rigid_body, collider);
        helmet_entry.add_component(collider);
        helmet_entry.add_component(rigid_body);
    }

    // Add the left hand
    let left_hand = add_model_to_world(
        "Left Hand",
        &models,
        &mut world,
        None,
        &vulkan_context,
        &render_context.descriptor_set_layouts,
    )
    .unwrap();
    {
        let mut left_hand_entry = world.entry(left_hand).unwrap();

        // Add a hand component
        left_hand_entry.add_component(Hand::left());

        // Modify the animation controller
        let animation_controller = left_hand_entry
            .get_component_mut::<AnimationController>()
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
        let (collider, rigid_body) =
            physics_context.get_rigid_body_and_collider(left_hand, rigid_body, collider);
        left_hand_entry.add_component(collider);
        left_hand_entry.add_component(rigid_body);
    }

    // Add the right hand
    let right_hand = add_model_to_world(
        "Right Hand",
        &models,
        &mut world,
        None,
        &vulkan_context,
        &render_context.descriptor_set_layouts,
    )
    .unwrap();
    {
        let mut right_hand_entry = world.entry(right_hand).unwrap();
        right_hand_entry.add_component(Hand::right());
        let animation_controller = right_hand_entry
            .get_component_mut::<AnimationController>()
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
        let (collider, rigid_body) =
            physics_context.get_rigid_body_and_collider(right_hand, rigid_body, collider);
        right_hand_entry.add_component(collider);
        right_hand_entry.add_component(rigid_body);
    }

    let mut resources = Resources::default();
    resources.insert(xr_context);
    resources.insert(vulkan_context);
    resources.insert(render_context);
    resources.insert(physics_context);
    resources.insert(0 as usize);
    let schedule = Schedule::builder()
        .add_thread_local_fn(begin_frame)
        .add_system(hands_system())
        .add_system(collision_system())
        .add_system(grabbing_system())
        .add_thread_local_fn(physics_step)
        .add_system(update_rigid_body_transforms_system())
        .add_system(animation_system())
        .add_system(update_transform_matrix_system())
        .add_system(update_parent_transform_matrix_system())
        .add_system(skinning_system())
        .add_system(rendering_system())
        .add_thread_local_fn(end_frame)
        .build();
    println!("[HOTHAM_INIT] DONE! INIT COMPLETE!");

    let mut app = App::new(world, resources, schedule)?;
    app.run()?;
    Ok(())
}
