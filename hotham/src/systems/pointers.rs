use ash::vk;
use egui::Pos2;
use legion::{system, world::SubWorld, IntoQuery};
use nalgebra::{
    point, vector, Isometry3, Orthographic3, Point3, Quaternion, Translation3, UnitQuaternion,
};
use rapier3d::{
    math::Point,
    prelude::{InteractionGroups, Ray},
};

const POSITION_OFFSET: [f32; 3] = [0., 0.071173, -0.066082];
const ROTATION_OFFSET: Quaternion<f32> = Quaternion::new(
    -0.5581498959847122,
    0.8274912503663805,
    0.03413791007514528,
    -0.05061153302400824,
);

use crate::{
    components::{
        hand::Handedness,
        panel::{get_panel_dimensions, PanelInput},
        Panel, Pointer, Transform,
    },
    resources::{gui_context::SCALE_FACTOR, PhysicsContext, XrContext},
    util::{posef_to_isometry, u64_to_entity},
};

#[system(for_each)]
#[write_component(Panel)]
pub fn pointers(
    pointer: &mut Pointer,
    transform: &mut Transform,
    world: &mut SubWorld,
    #[resource] xr_context: &XrContext,
    #[resource] physics_context: &mut PhysicsContext,
) {
    // Get our the space and path of the pointer.
    let time = xr_context.frame_state.predicted_display_time;
    let (space, path) = match pointer.handedness {
        Handedness::Left => (
            &xr_context.left_hand_space,
            xr_context.left_hand_subaction_path,
        ),
        Handedness::Right => (
            &xr_context.right_hand_space,
            xr_context.right_hand_subaction_path,
        ),
    };

    // Locate the pointer in the space.
    let pose = space
        .locate(&xr_context.reference_space, time)
        .unwrap()
        .pose;

    // apply transform
    let mut position = posef_to_isometry(pose);
    apply_grip_offset(&mut position);

    transform.translation = position.translation.vector;
    transform.rotation = position.rotation;

    // get trigger value
    let trigger_value =
        openxr::ActionInput::get(&xr_context.trigger_action, &xr_context.session, path)
            .unwrap()
            .current_state;
    pointer.trigger_value = trigger_value;

    let ray_direction = transform.rotation.transform_vector(&vector![0., 1.0, 0.]);

    // Sweet baby ray
    let ray = Ray::new(Point::from(transform.translation), ray_direction);
    let max_toi = 40.0;
    let solid = true;
    let groups = InteractionGroups::new(0b10, 0b10);
    let filter = None;

    if let Some((handle, toi)) = physics_context.query_pipeline.cast_ray(
        &physics_context.colliders,
        &ray,
        max_toi,
        solid,
        groups,
        filter,
    ) {
        // The first collider hit has the handle `handle` and it hit after
        // the ray travelled a distance equal to `ray.dir * toi`.
        let hit_point = ray.point_at(toi); // Same as: `ray.origin + ray.dir * toi`
        let hit_collider = physics_context.colliders.get(handle).unwrap();
        let entity = u64_to_entity(hit_collider.user_data as u64);
        let mut query = <&mut Panel>::query();
        let panel = query
            .get_mut(world, entity)
            .expect(&format!("Unable to find entity {:?} in world", entity));
        let panel_extent = &panel.extent;
        let panel_transform = hit_collider.position();
        let cursor_location =
            get_cursor_location_for_panel(&hit_point, panel_transform, panel_extent);
        panel.input = Some(PanelInput {
            cursor_location,
            trigger_value,
        });
    }
}

pub fn apply_grip_offset(position: &mut Isometry3<f32>) {
    let updated_rotation = position.rotation.quaternion() * ROTATION_OFFSET;
    let updated_translation = position.translation.vector
        - vector!(POSITION_OFFSET[0], POSITION_OFFSET[1], POSITION_OFFSET[2]);
    position.rotation = UnitQuaternion::from_quaternion(updated_rotation);
    position.translation = Translation3::from(updated_translation);
}

fn get_cursor_location_for_panel(
    hit_point: &Point3<f32>,
    panel_transform: &Isometry3<f32>,
    panel_extent: &vk::Extent2D,
) -> Pos2 {
    let projected_hit_point = ray_to_panel_space(hit_point, panel_transform, panel_extent);
    let transformed_hit_point = panel_transform
        .rotation
        .transform_point(&projected_hit_point);

    // Adjust the point such that 0,0 is the panel's top left
    let x = (transformed_hit_point.x + 1.) * 0.5;
    let y = ((transformed_hit_point.y * -1.) * 0.5) + 0.5;

    // Convert to screen coordinates
    let x_points = x * panel_extent.width as f32 / SCALE_FACTOR;
    let y_points = y * panel_extent.height as f32 / SCALE_FACTOR;

    return Pos2::new(x_points, y_points);
}

fn ray_to_panel_space(
    hit_point: &Point3<f32>,
    panel_transform: &Isometry3<f32>,
    panel_extent: &vk::Extent2D,
) -> Point3<f32> {
    // Translate the extents of the panel into world space, using the panel's translation.
    let (extent_x, extent_y) = get_panel_dimensions(&panel_extent);
    let translated_extents = panel_transform * point![extent_x, extent_y, 0.];

    // Now build an orthographic matrix to project from world space into the panel's screen space
    let left = translated_extents.x - 1.;
    let right = translated_extents.x;
    let bottom = translated_extents.y - 1.;
    let top = translated_extents.y;
    let panel_projection = Orthographic3::new(left, right, bottom, top, 0., 1.);

    // Project the ray's hit point into panel space
    return panel_projection.project_point(hit_point);
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use approx::assert_relative_eq;
    use ash::vk;
    use legion::{IntoQuery, Resources, Schedule, World};
    use nalgebra::vector;
    use rapier3d::prelude::ColliderBuilder;

    use crate::{
        buffer::Buffer,
        components::{Collider, Panel, Transform},
        resources::{
            physics_context::{DEFAULT_COLLISION_GROUP, PANEL_COLLISION_GROUP},
            XrContext,
        },
        schedule_functions::physics_step,
        util::entity_to_u64,
    };

    use super::*;
    #[test]
    pub fn test_pointers_system() {
        let (xr_context, _) = XrContext::new().unwrap();
        let mut physics_context = PhysicsContext::default();
        let mut world = World::default();
        let mut resources = Resources::default();

        let panel = Panel {
            text: "Test Panel".to_string(),
            extent: vk::Extent2D {
                width: 300,
                height: 300,
            },
            framebuffer: vk::Framebuffer::null(),
            vertex_buffer: empty_buffer(),
            index_buffer: empty_buffer(),
            egui_context: Default::default(),
            raw_input: Default::default(),
            input: Default::default(),
            buttons: Vec::new(),
        };
        let panel_entity = world.push((panel,));

        // Place the panel *directly above* where the pointer will be located.
        let collider = ColliderBuilder::cuboid(0.5, 0.5, 0.0)
            .sensor(true)
            .collision_groups(InteractionGroups::new(
                PANEL_COLLISION_GROUP,
                PANEL_COLLISION_GROUP,
            ))
            .translation(vector![-0.2, 2., -0.433918])
            .rotation(vector![(3. * std::f32::consts::PI) * 0.5, 0., 0.])
            .user_data(entity_to_u64(panel_entity).into())
            .build();
        let handle = physics_context.colliders.insert(collider);
        let collider = Collider {
            collisions_this_frame: Vec::new(),
            handle,
        };
        let mut panel_entry = world.entry(panel_entity).unwrap();
        panel_entry.add_component(collider);

        // Add a decoy collider to ensure we're using collision groups correctly.
        let collider = ColliderBuilder::cuboid(0.1, 0.1, 0.1)
            .sensor(true)
            .collision_groups(InteractionGroups::new(
                DEFAULT_COLLISION_GROUP,
                DEFAULT_COLLISION_GROUP,
            ))
            .translation(vector![-0.2, 1.5, -0.433918])
            .rotation(vector![(3. * std::f32::consts::PI) * 0.5, 0., 0.])
            .build();
        let handle = physics_context.colliders.insert(collider);
        let collider = Collider {
            collisions_this_frame: Vec::new(),
            handle,
        };
        world.push((collider,));

        resources.insert(xr_context);
        resources.insert(physics_context);
        let pointer_entity = world.push((
            Pointer {
                handedness: Handedness::Left,
                trigger_value: 0.0,
            },
            Transform::default(),
        ));

        let mut schedule = Schedule::builder()
            .add_thread_local_fn(physics_step)
            .add_system(pointers_system())
            .build();

        schedule.execute(&mut world, &mut resources);
        let mut query = <&Transform>::query();
        let transform = query.get(&world, pointer_entity).unwrap();

        // Assert that the pointer has moved
        assert_relative_eq!(transform.translation, vector![-0.2, 1.328827, -0.433918]);

        let mut query = <&Panel>::query();
        let panel = query.get(&world, panel_entity).unwrap();
        let input = panel.input.clone().unwrap();
        assert_relative_eq!(input.cursor_location.x, 50.);
        assert_relative_eq!(input.cursor_location.y, 29.491043);
        assert_eq!(input.trigger_value, 0.);
    }

    #[test]
    pub fn test_get_cursor_location_for_panel() {
        let panel_transform = Isometry3::new(nalgebra::zero(), nalgebra::zero());
        let panel_extent = vk::Extent2D {
            width: 100 * SCALE_FACTOR as u32,
            height: 100 * SCALE_FACTOR as u32,
        };

        // Trivial example. Panel and hit point at origin:
        let result =
            get_cursor_location_for_panel(&point![0., 0., 0.], &panel_transform, &panel_extent);
        assert_relative_eq!(result.x, 50.);
        assert_relative_eq!(result.y, 50.);

        // hit top left
        let result =
            get_cursor_location_for_panel(&point![-0.5, 0.5, 0.], &panel_transform, &panel_extent);
        assert_relative_eq!(result.x, 0.);
        assert_relative_eq!(result.y, 0.);

        // hit top right
        let result =
            get_cursor_location_for_panel(&point![0.5, 0.5, 0.], &panel_transform, &panel_extent);
        assert_relative_eq!(result.x, 100.);
        assert_relative_eq!(result.y, 0.);

        // hit bottom right
        let result =
            get_cursor_location_for_panel(&point![0.5, -0.5, 0.], &panel_transform, &panel_extent);
        assert_relative_eq!(result.x, 100.);
        assert_relative_eq!(result.y, 100.);

        // hit bottom left
        let result =
            get_cursor_location_for_panel(&point![-0.5, -0.5, 0.], &panel_transform, &panel_extent);
        assert_relative_eq!(result.x, 0.);
        assert_relative_eq!(result.y, 100.);
    }

    #[test]
    pub fn test_ray_to_panel_space() {
        let panel_transform = Isometry3::new(nalgebra::zero(), nalgebra::zero());
        let panel_extent = vk::Extent2D {
            width: 100,
            height: 100,
        };

        let result = ray_to_panel_space(&point![0., 0., 0.], &panel_transform, &panel_extent);
        assert_relative_eq!(result, point![0.0, 0.0, -1.0]);

        // hit top left
        let result = ray_to_panel_space(&point![-0.5, 0.5, 0.], &panel_transform, &panel_extent);
        assert_relative_eq!(result, point![-1.0, 1.0, -1.0]);

        // hit top right
        let result = ray_to_panel_space(&point![0.5, 0.5, 0.], &panel_transform, &panel_extent);
        assert_relative_eq!(result, point![1.0, 1.0, -1.0]);

        // hit bottom right
        let result = ray_to_panel_space(&point![0.5, -0.5, 0.], &panel_transform, &panel_extent);
        assert_relative_eq!(result, point![1.0, -1.0, -1.0]);

        // hit bottom left
        let result = ray_to_panel_space(&point![-0.5, -0.5, 0.], &panel_transform, &panel_extent);
        assert_relative_eq!(result, point![-1.0, -1.0, -1.0]);
    }

    fn empty_buffer<T>() -> Buffer<T> {
        let vertex_buffer = Buffer {
            handle: vk::Buffer::null(),
            device_memory: vk::DeviceMemory::null(),
            _phantom: PhantomData,
            size: 0,
            device_memory_size: 0,
            usage: vk::BufferUsageFlags::empty(),
        };
        vertex_buffer
    }
}
