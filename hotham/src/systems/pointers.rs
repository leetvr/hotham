use ash::vk;
use egui::Pos2;
use hecs::{With, World};
use nalgebra::{
    point, vector, Isometry3, Orthographic3, Point3, Quaternion, Translation3, UnitQuaternion,
    Vector2,
};
use rapier3d::{
    math::Point,
    prelude::{InteractionGroups, QueryFilter, Ray},
};

const POSITION_OFFSET: Translation3<f32> = Translation3::new(0., -0.08, 0.);

/* Original Precision
    -0.5581498959847122,
    0.8274912503663805,
    0.03413791007514528,
    -0.05061153302400824,
*/
const ROTATION_OFFSET: UnitQuaternion<f32> = UnitQuaternion::new_unchecked(Quaternion::<f32>::new(
    -0.558_149_9,
    0.827_491_2,
    0.034_137_91,
    -0.050_611_533,
));

use crate::{
    components::{
        hand::Handedness, panel::PanelInput, Info, LocalTransform, Panel, Pointer, Visible,
    },
    contexts::{InputContext, PhysicsContext},
    Engine,
};

use super::stage;

/// Pointers system
/// Allows users to interact with `Panel`s using their controllers
pub fn pointers_system(engine: &mut Engine) {
    let world = &mut engine.world;
    let input_context = &mut engine.input_context;
    let physics_context = &mut engine.physics_context;

    pointers_system_inner(world, input_context, physics_context);
}

pub fn pointers_system_inner(
    world: &mut World,
    input_context: &InputContext,
    physics_context: &mut PhysicsContext,
) {
    // Get the isometry of the stage
    let global_from_stage = stage::get_global_from_stage(world);

    // Create a transform from local space to grip space.
    let grip_from_local = ROTATION_OFFSET * POSITION_OFFSET;

    for (_, (pointer, local_transform)) in world
        .query::<With<Visible, (&mut Pointer, &mut LocalTransform)>>()
        .iter()
    {
        // Get the position of the pointer in stage space.
        let (stage_from_grip, trigger_value) = match pointer.handedness {
            Handedness::Left => (
                input_context.left.stage_from_grip(),
                input_context.left.trigger_analog(),
            ),
            Handedness::Right => (
                input_context.right.stage_from_grip(),
                input_context.right.trigger_analog(),
            ),
        };
        println!("Grip position is {:?}", stage_from_grip);

        // Compose transform
        let global_from_local = global_from_stage * stage_from_grip * grip_from_local;
        println!("Setting saber position to {:?}", global_from_local);
        local_transform.update_from_isometry(&global_from_local);
        println!("Saber position is {:?}", local_transform.translation);

        // Get trigger value
        pointer.trigger_value = trigger_value;

        let ray_direction = local_transform
            .rotation
            .transform_vector(&vector![0., 1.0, 0.]);

        // Sweet baby ray
        let ray = Ray::new(Point::from(local_transform.translation), ray_direction);
        let max_toi = 40.0;
        let solid = true;
        let groups = InteractionGroups::new(0b10, 0b10);
        let filter = QueryFilter::new().groups(groups);

        if let Some((handle, toi)) = physics_context.query_pipeline.cast_ray(
            &physics_context.rigid_bodies,
            &physics_context.colliders,
            &ray,
            max_toi,
            solid,
            filter,
        ) {
            // The first collider hit has the handle `handle` and it hit after
            // the ray traveled a distance equal to `ray.dir * toi`.
            let hit_point = ray.point_at(toi); // Same as: `ray.origin + ray.dir * toi`
            let hit_collider = physics_context.colliders.get(handle).unwrap();
            let entity = unsafe { world.find_entity_from_id(hit_collider.user_data as _) };
            match world.get_mut::<Panel>(entity) {
                Ok(mut panel) => {
                    let panel_transform = hit_collider.position();
                    let cursor_location = get_cursor_location_for_panel(
                        &hit_point,
                        panel_transform,
                        &panel.resolution,
                        &panel.world_size,
                    );
                    panel.input = Some(PanelInput {
                        cursor_location,
                        trigger_value,
                    });
                }
                Err(_) => {
                    let info = world.get::<Info>(entity).map(|i| format!("{:?}", *i));
                    println!("[HOTHAM_POINTERS] Ray collided with object that does not have a panel: {:?} - {:?}", entity, info);
                }
            }
        }
    }
}

fn get_cursor_location_for_panel(
    hit_point: &Point3<f32>,
    panel_transform: &Isometry3<f32>,
    panel_extent: &vk::Extent2D,
    panel_world_size: &Vector2<f32>,
) -> Pos2 {
    let projected_hit_point = ray_to_panel_space(hit_point, panel_transform, panel_world_size);
    let transformed_hit_point = panel_transform
        .rotation
        .transform_point(&projected_hit_point);

    // Adjust the point such that 0,0 is the panel's top left
    let x = (transformed_hit_point.x + 1.) * 0.5;
    let y = ((transformed_hit_point.y * -1.) * 0.5) + 0.5;

    // Convert to screen coordinates
    let x_points = x * panel_extent.width as f32;
    let y_points = y * panel_extent.height as f32;

    Pos2::new(x_points, y_points)
}

fn ray_to_panel_space(
    hit_point: &Point3<f32>,
    panel_transform: &Isometry3<f32>,
    panel_world_size: &Vector2<f32>,
) -> Point3<f32> {
    // Translate the extents of the panel into world space, using the panel's translation.
    let (extent_x, extent_y) = (panel_world_size.x / 2., panel_world_size.y / 2.);
    let translated_extents = panel_transform * point![extent_x, extent_y, 0.];

    // Now build an orthographic matrix to project from world space into the panel's screen space
    let left = translated_extents.x - 1.;
    let right = translated_extents.x;
    let bottom = translated_extents.y - 1.;
    let top = translated_extents.y;
    let panel_projection = Orthographic3::new(left, right, bottom, top, 0., 1.);

    // Project the ray's hit point into panel space
    panel_projection.project_point(hit_point)
}

#[cfg(test)]
mod tests {
    use super::*;

    use approx::assert_relative_eq;
    use ash::vk;

    #[cfg(target_os = "windows")]
    #[test]
    pub fn test_pointers_system() {
        use crate::{
            components::{Collider, LocalTransform, Panel},
            contexts::{
                physics_context::{DEFAULT_COLLISION_GROUP, PANEL_COLLISION_GROUP},
                RenderContext,
            },
        };
        use nalgebra::vector;
        use rapier3d::prelude::ColliderBuilder;
        const POINTER_Z: f32 = -0.47001815;

        let (mut render_context, vulkan_context) = RenderContext::testing();
        let mut physics_context = PhysicsContext::default();
        let input_context = InputContext::testing();
        let mut world = World::default();

        let panel = Panel::create(
            &vulkan_context,
            &mut render_context,
            vk::Extent2D {
                width: 300,
                height: 300,
            },
            [1.0, 1.0].into(),
        )
        .unwrap();
        let panel_entity = world.spawn(panel);

        // Place the panel *directly above* where the pointer will be located.
        let collider = ColliderBuilder::cuboid(0.5, 0.5, 0.0)
            .sensor(true)
            .collision_groups(InteractionGroups::new(
                PANEL_COLLISION_GROUP,
                PANEL_COLLISION_GROUP,
            ))
            .translation([-0.2, 2., POINTER_Z].into())
            .rotation(vector![(3. * std::f32::consts::PI) * 0.5, 0., 0.])
            .user_data(panel_entity.id() as _)
            .build();
        let handle = physics_context.colliders.insert(collider);
        let collider = Collider {
            collisions_this_frame: Vec::new(),
            handle,
        };
        world.insert_one(panel_entity, collider).unwrap();

        // Add a decoy collider to ensure we're using collision groups correctly.
        let collider = ColliderBuilder::cuboid(0.1, 0.1, 0.1)
            .sensor(true)
            .collision_groups(InteractionGroups::new(
                DEFAULT_COLLISION_GROUP,
                DEFAULT_COLLISION_GROUP,
            ))
            .translation([-0.2, 1.5, POINTER_Z].into())
            .rotation(vector![(3. * std::f32::consts::PI) * 0.5, 0., 0.])
            .build();
        let handle = physics_context.colliders.insert(collider);
        let collider = Collider {
            collisions_this_frame: Vec::new(),
            handle,
        };
        world.spawn((collider,));

        let pointer_entity = world.spawn((
            Visible {},
            Pointer {
                handedness: Handedness::Left,
                trigger_value: 0.0,
            },
            LocalTransform::default(),
        ));

        tick(&mut physics_context, &mut world, &input_context);

        let local_transform = world.get::<LocalTransform>(pointer_entity).unwrap();

        // Assert that the pointer has moved
        assert_relative_eq!(
            local_transform.translation,
            [-0.2, 1.3258567, POINTER_Z].into()
        );

        let panel = world.get_mut::<Panel>(panel_entity).unwrap();
        let input = panel.input.clone().unwrap();
        assert_relative_eq!(input.cursor_location.x, 150.);
        assert_relative_eq!(input.cursor_location.y, 77.30967);
        assert_eq!(input.trigger_value, 0.);
    }

    #[cfg(target_os = "windows")]
    fn tick(
        physics_context: &mut PhysicsContext,
        world: &mut hecs::World,
        input_context: &InputContext,
    ) {
        use crate::systems::physics::physics_system_inner;

        physics_system_inner(physics_context, world);
        pointers_system_inner(world, input_context, physics_context);
    }

    #[test]
    pub fn test_get_cursor_location_for_panel() {
        let panel_transform = Isometry3::new(nalgebra::zero(), nalgebra::zero());
        let panel_extent = vk::Extent2D {
            width: 100,
            height: 100,
        };
        let panel_world_size = vector![1.0, 1.0];

        // Trivial example. Panel and hit point at origin:
        let result = get_cursor_location_for_panel(
            &point![0., 0., 0.],
            &panel_transform,
            &panel_extent,
            &panel_world_size,
        );
        assert_relative_eq!(result.x, 50.);
        assert_relative_eq!(result.y, 50.);

        // hit top left
        let result = get_cursor_location_for_panel(
            &point![-0.5, 0.5, 0.],
            &panel_transform,
            &panel_extent,
            &panel_world_size,
        );
        assert_relative_eq!(result.x, 0.);
        assert_relative_eq!(result.y, 0.);

        // hit top right
        let result = get_cursor_location_for_panel(
            &point![0.5, 0.5, 0.],
            &panel_transform,
            &panel_extent,
            &panel_world_size,
        );
        assert_relative_eq!(result.x, 100.);
        assert_relative_eq!(result.y, 0.);

        // hit bottom right
        let result = get_cursor_location_for_panel(
            &point![0.5, -0.5, 0.],
            &panel_transform,
            &panel_extent,
            &panel_world_size,
        );
        assert_relative_eq!(result.x, 100.);
        assert_relative_eq!(result.y, 100.);

        // hit bottom left
        let result = get_cursor_location_for_panel(
            &point![-0.5, -0.5, 0.],
            &panel_transform,
            &panel_extent,
            &panel_world_size,
        );
        assert_relative_eq!(result.x, 0.);
        assert_relative_eq!(result.y, 100.);
    }

    #[test]
    pub fn test_ray_to_panel_space() {
        let panel_transform = Isometry3::new(nalgebra::zero(), nalgebra::zero());
        let panel_world_size = vector![1.0, 1.0];

        let result = ray_to_panel_space(&point![0., 0., 0.], &panel_transform, &panel_world_size);
        assert_relative_eq!(result, point![0.0, 0.0, -1.0]);

        // hit top left
        let result =
            ray_to_panel_space(&point![-0.5, 0.5, 0.], &panel_transform, &panel_world_size);
        assert_relative_eq!(result, point![-1.0, 1.0, -1.0]);

        // hit top right
        let result = ray_to_panel_space(&point![0.5, 0.5, 0.], &panel_transform, &panel_world_size);
        assert_relative_eq!(result, point![1.0, 1.0, -1.0]);

        // hit bottom right
        let result =
            ray_to_panel_space(&point![0.5, -0.5, 0.], &panel_transform, &panel_world_size);
        assert_relative_eq!(result, point![1.0, -1.0, -1.0]);

        // hit bottom left
        let result =
            ray_to_panel_space(&point![-0.5, -0.5, 0.], &panel_transform, &panel_world_size);
        assert_relative_eq!(result, point![-1.0, -1.0, -1.0]);
    }
}
