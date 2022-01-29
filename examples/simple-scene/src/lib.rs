use hotham::components::hand::Handedness;
use hotham::components::{
    AnimationTarget, Collider, Info, Joint, Mesh, Parent, RigidBody, Skin, TransformMatrix, Visible,
};
use hotham::gltf_loader::add_model_to_world;
use hotham::hecs::Without;
use hotham::resources::vulkan_context::VulkanContext;
use hotham::resources::{RenderContext, XrContext};
use hotham::schedule_functions::{
    begin_frame, begin_pbr_renderpass, end_frame, end_pbr_renderpass, physics_step,
};
use hotham::systems::rendering::rendering_system;
use hotham::systems::skinning::skinning_system;
use hotham::systems::{
    animation_system, collision_system, grabbing_system, hands_system,
    update_parent_transform_matrix_system, update_rigid_body_transforms_system,
    update_transform_matrix_system,
};
use hotham::{gltf_loader, Engine, HothamError, HothamResult};

use hotham::{
    components::{AnimationController, Hand, Transform},
    hecs::{PreparedQuery, With, World},
    rapier3d::prelude::{ActiveCollisionTypes, ActiveEvents, ColliderBuilder, RigidBodyBuilder},
    resources::PhysicsContext,
};

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
pub fn main() {
    println!("[HOTHAM_SIMPLE_SCENE] MAIN!");
    real_main().expect("Error running app!");
}

pub fn real_main() -> HothamResult<()> {
    let mut engine = Engine::new();
    let mut resources = init()?;
    let mut queries = Default::default();

    while !engine.should_quit() {
        match engine.update(&mut resources.xr_context) {
            Err(HothamError::ShuttingDown) => {
                println!("[HOTHAM_SIMPLE_SCENE] Shutting down!");
                break;
            }
            Err(e) => return Err(e),
            _ => {}
        }
        tick(&mut resources, &mut queries);
    }

    Ok(())
}

fn init() -> Result<Resources, hotham::HothamError> {
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
    let helmet = add_model_to_world(
        "Damaged Helmet",
        &models,
        &mut world,
        None,
        &vulkan_context,
        &render_context.descriptor_set_layouts,
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
    add_hand(
        &models,
        Handedness::Left,
        &mut world,
        &vulkan_context,
        &render_context,
        &mut physics_context,
    );
    add_hand(
        &models,
        Handedness::Right,
        &mut world,
        &vulkan_context,
        &render_context,
        &mut physics_context,
    );

    Ok(Resources {
        xr_context,
        vulkan_context,
        render_context,
        physics_context,
        world,
    })
}

struct Resources {
    xr_context: XrContext,
    vulkan_context: VulkanContext,
    render_context: RenderContext,
    physics_context: PhysicsContext,
    world: World,
}

#[derive(Default)]
struct Queries<'a> {
    collision_query: PreparedQuery<&'a mut Collider>,
    grabbing_query: PreparedQuery<(&'a mut Hand, &'a Collider)>,
    update_rigid_body_transforms_query: PreparedQuery<(&'a RigidBody, &'a mut Transform)>,
    animation_query: PreparedQuery<(&'a mut AnimationTarget, &'a mut Transform)>,
    update_transform_matrix_query: PreparedQuery<(&'a Transform, &'a mut TransformMatrix)>,
    parent_query: PreparedQuery<&'a Parent>,
    roots_query: PreparedQuery<Without<Parent, &'a TransformMatrix>>,
    joints_query: PreparedQuery<(&'a TransformMatrix, &'a Joint, &'a Info)>,
    meshes_query: PreparedQuery<(&'a mut Mesh, &'a Skin)>,
    rendering_query: PreparedQuery<With<Visible, (&'a mut Mesh, &'a TransformMatrix)>>,
    hands_query: PreparedQuery<(
        &'a mut Hand,
        &'a mut AnimationController,
        &'a mut hotham::components::RigidBody,
    )>,
}

fn tick(resources: &mut Resources, queries: &mut Queries) {
    let xr_context = &mut resources.xr_context;
    let vulkan_context = &resources.vulkan_context;
    let render_context = &mut resources.render_context;
    let physics_context = &mut resources.physics_context;
    let world = &mut resources.world;

    begin_frame(xr_context, vulkan_context, render_context);
    hands_system(&mut queries.hands_query, world, xr_context, physics_context);
    collision_system(&mut queries.collision_query, world, physics_context);
    grabbing_system(&mut queries.grabbing_query, world, physics_context);
    physics_step(physics_context);
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
    begin_pbr_renderpass(xr_context, vulkan_context, render_context);
    rendering_system(
        &mut queries.rendering_query,
        world,
        vulkan_context,
        xr_context.frame_index,
        render_context,
    );
    end_pbr_renderpass(xr_context, vulkan_context, render_context);
    end_frame(xr_context, vulkan_context, render_context);
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
    let hand = add_model_to_world(
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
        world
            .insert_one(
                hand,
                Hand {
                    grip_value: 0.,
                    handedness,
                    grabbed_entity: None,
                },
            )
            .unwrap();

        // Modify the animation controller
        let mut animation_controller = world.get_mut::<AnimationController>(hand).unwrap();
        animation_controller.blend_from = 0;
        animation_controller.blend_to = 1;
        drop(animation_controller);

        // Give it a collider and rigid-body
        let collider = ColliderBuilder::capsule_y(0.05, 0.02)
            .sensor(true)
            .active_collision_types(ActiveCollisionTypes::all())
            .active_events(ActiveEvents::CONTACT_EVENTS | ActiveEvents::INTERSECTION_EVENTS)
            .build();
        let rigid_body = RigidBodyBuilder::new_kinematic_position_based().build();
        let components = physics_context.get_rigid_body_and_collider(hand, rigid_body, collider);
        world.insert(hand, components).unwrap();
    }
}
