use crate::{
    components::{GlobalTransform, Hand, LocalTransform, Parent, Stage},
    hecs::{CommandBuffer, Entity, With, World},
    util::{isometry_to_posef, matrix_to_isometry, posef_to_isometry},
    xr,
};

/// Setup Stage entities to track player's frame of reference in global space
///
/// This should be run after `add_hand`.
pub fn add_stage(world: &mut World) -> Entity {
    // Add Stage entity to track the position of the player's space with respect to the game-world
    let stage_e = world.spawn((
        Stage {},
        GlobalTransform::default(),
        LocalTransform::default(),
    ));

    // Make hands children of Stage
    let mut cmd_buffer = CommandBuffer::new();
    for (hand_e, _) in world.query_mut::<&Hand>() {
        cmd_buffer.insert(hand_e, (Parent(stage_e),));
    }
    cmd_buffer.run_on(world);

    stage_e
}

/// Update player's views to take into account the current position of the Stage in global space
///
/// Must happen each tick after parent transforms have been updated.
pub fn update_views_with_stage_transform(world: &mut World, views: &mut [xr::View]) {
    let stage_isometry = world
        .query_mut::<With<Stage, &GlobalTransform>>()
        .into_iter()
        .next()
        .map(|(_, global_transform)| matrix_to_isometry(global_transform.0));

    if let Some(stage_isometry) = stage_isometry {
        for view in views {
            view.pose = isometry_to_posef(stage_isometry * posef_to_isometry(view.pose));
        }
    }
}
