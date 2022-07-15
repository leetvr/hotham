use hotham::{
    asset_importer::add_model_to_world,
    components::RigidBody,
    hecs::{PreparedQuery, World},
    rapier3d::prelude::{
        ActiveCollisionTypes, ActiveEvents, ColliderBuilder, RigidBodyBuilder, RigidBodyType,
    },
    resources::{PhysicsContext, XrContext},
    util::{is_space_valid, posef_to_isometry},
};

pub enum AxesSpace {
    LeftHand,
    LeftPointer,
    RightHand,
    RightPointer,
}

pub struct Axes {
    space: AxesSpace,
}

pub fn add_axes(
    models: &std::collections::HashMap<String, World>,
    world: &mut World,
    physics_context: &mut PhysicsContext,
    space: AxesSpace,
) {
    let entity = add_model_to_world("Axes", models, world, None).unwrap();
    world.insert_one(entity, Axes { space }).unwrap();

    // Give it a collider and rigid-body
    let collider = ColliderBuilder::cuboid(0.02, 0.02, 0.02)
        .sensor(true)
        .active_collision_types(ActiveCollisionTypes::all())
        .active_events(ActiveEvents::COLLISION_EVENTS)
        .build();
    let rigid_body = RigidBodyBuilder::new(RigidBodyType::KinematicPositionBased).build();
    let components = physics_context.get_rigid_body_and_collider(entity, rigid_body, collider);
    world.insert(entity, components).unwrap();
}

pub fn axes_system(
    query: &mut PreparedQuery<(&mut Axes, &mut RigidBody)>,
    world: &mut World,
    xr_context: &XrContext,
    physics_context: &mut PhysicsContext,
) {
    let input = &xr_context.input;
    for (_, (axes, rigid_body_component)) in query.query(world).iter() {
        // Get the space and path of the hand.
        let time = xr_context.frame_state.predicted_display_time;
        let space = match axes.space {
            AxesSpace::LeftHand => &input.left_hand_space,
            AxesSpace::LeftPointer => &input.left_pointer_space,
            AxesSpace::RightHand => &input.right_hand_space,
            AxesSpace::RightPointer => &input.right_pointer_space,
        };

        // Locate the space relative to the stage.
        let stage_from_space = space.locate(&xr_context.stage_space, time).unwrap();

        // Check it's valid before using it
        if !is_space_valid(&stage_from_space) {
            continue;
        }

        let stage_from_space = stage_from_space.pose;

        // Apply transform
        let rigid_body = physics_context
            .rigid_bodies
            .get_mut(rigid_body_component.handle)
            .unwrap();

        rigid_body.set_next_kinematic_position(posef_to_isometry(stage_from_space));
    }
}
