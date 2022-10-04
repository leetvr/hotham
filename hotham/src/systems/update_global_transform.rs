use crate::{
    components::{GlobalTransform, LocalTransform},
    Engine,
};
use hecs::World;

/// Update global transform matrix system
/// Walks through each LocalTransform and applies it to a 4x4 matrix used by the vertex shader
pub fn update_global_transform_system(engine: &mut Engine) {
    let world = &mut engine.world;
    update_global_transform_system_inner(world);
}

pub(crate) fn update_global_transform_system_inner(world: &mut World) {
    for (_, (local_transform, global_transform)) in
        world.query_mut::<(&LocalTransform, &mut GlobalTransform)>()
    {
        global_transform.0 = local_transform.to_affine();
    }
}

#[cfg(test)]
mod tests {
    use crate::components::LocalTransform;
    use approx::assert_relative_eq;
    use glam::{Affine3A, EulerRot, Quat};

    use super::*;

    #[test]
    pub fn test_update_global_transform_system() {
        let mut world = World::new();
        let local_transform = LocalTransform::default();
        let global_transform = GlobalTransform::default();

        let entity = world.spawn((local_transform, global_transform));

        {
            let matrix = world.get::<&mut GlobalTransform>(entity).unwrap();
            assert_eq!(matrix.0, Affine3A::IDENTITY);
        }

        let test_translation = [5.0, 1.0, 2.0].into();
        let test_rotation = Quat::from_euler(EulerRot::XYZ, 0.3, 0.3, 0.3);

        {
            let mut local_transform = world.get::<&mut LocalTransform>(entity).unwrap();
            local_transform.translation = test_translation;
            local_transform.rotation = test_rotation;
            local_transform.scale = test_translation;
        }

        update_global_transform_system_inner(&mut world);

        let expected_matrix = Affine3A::from_scale_rotation_translation(
            [5.0, 1.0, 2.0].into(),
            test_rotation,
            test_translation,
        );

        let global_transform = world.get::<&mut GlobalTransform>(entity).unwrap();
        assert_relative_eq!(global_transform.0, expected_matrix);
    }
}
