use crate::{
    components::{Hand, Parent, Room, Transform, TransformMatrix},
    hecs::{CommandBuffer, Entity, With, World},
    util::{isometry_to_posef, matrix_to_isometry, posef_to_isometry},
    xr,
};

/// Setup Room and Hmd entities to track player's frame of reference in the world
///
/// This should be run after `add_hand`.
pub fn add_room(world: &mut World) -> Entity {
    // Add Room entity to track the position of the player's space with respect to the game-world
    let room_e = world.spawn((Room {}, TransformMatrix::default(), Transform::default()));

    // Make hands children of Room
    let mut cmd_buffer = CommandBuffer::new();
    for (hand_e, _) in world.query_mut::<&Hand>() {
        cmd_buffer.insert(hand_e, (Parent(room_e),));
    }
    cmd_buffer.run_on(world);

    room_e
}

/// Update player's views to take into account the current position of the Room in the world
///
/// Must happen each tick after parent transforms have been updated.
pub fn update_views_with_room_transform(world: &mut World, views: &mut [xr::View]) {
    let room_isometry = world
        .query_mut::<With<Room, &TransformMatrix>>()
        .into_iter()
        .next()
        .map(|(_, matrix)| matrix_to_isometry(matrix.0));

    if let Some(room_isometry) = room_isometry {
        for view in views {
            view.pose = isometry_to_posef(room_isometry * posef_to_isometry(view.pose));
        }
    }
}
