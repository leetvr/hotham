use cgmath::{Matrix4, SquareMatrix};
use legion::{system, world::SubWorld, Entity, EntityStore, IntoQuery};
use std::collections::HashMap;

use crate::{
    components::{Joint, Skin, Transform},
    resources::VulkanContext,
};

#[system]
#[read_component(Joint)]
#[read_component(Transform)]
#[read_component(Skin)]
pub(crate) fn skinning(world: &mut SubWorld, #[resource] vulkan_context: &VulkanContext) -> () {
    let mut joint_matrices: HashMap<Entity, Vec<Matrix4<f32>>> = HashMap::new();
    unsafe {
        let mut query = <(&Transform, &Joint)>::query();
        query.for_each_unchecked(world, |(transform, joint)| {
            let parent = transform.parent.unwrap();
            let parent = world.entry_ref(parent).unwrap();
            let parent_transform = parent.get_component::<Transform>().unwrap();

            let skeleton_root = joint.skeleton_root;
            let inverse_transform = parent_transform.global_matrix.invert().unwrap();

            let joint_matrix =
                inverse_transform * transform.global_matrix * joint.inverse_bind_matrix;
            let matrices = joint_matrices.entry(skeleton_root).or_default();
            matrices.push(joint_matrix);
        });
    }

    let mut query = <&Skin>::query();
    query.for_each_chunk(world, |chunk| {
        for (entity, skin) in chunk.into_iter_entities() {
            let matrices = joint_matrices.get(&entity).unwrap();
            let buffer = &skin.buffer;
            buffer
                .update(vulkan_context, matrices.as_ptr(), matrices.len())
                .unwrap();
        }
    });
}

#[cfg(test)]
mod tests {

    use crate::{
        buffer::Buffer,
        components::{Joint, Skin, Transform},
        resources::VulkanContext,
        systems::transform::{get_global_matrix, get_local_matrix},
    };

    use super::*;
    use ash::version::DeviceV1_0;
    use cgmath::{vec3, Matrix4, SquareMatrix};
    use legion::{Resources, Schedule, World};

    #[test]
    pub fn test_skinning_system() {
        let mut world = World::default();
        let vulkan_context = VulkanContext::testing().unwrap();

        // Create the transform for the skin entity
        let mut root_transform = Transform::default();
        root_transform.translation = vec3(1.0, 2.0, 3.0);
        root_transform.local_matrix = get_local_matrix(&root_transform);

        root_transform.global_matrix = get_local_matrix(&root_transform);
        // Create a skin
        let inverse = root_transform.local_matrix.invert().unwrap();
        let joint_matrices = vec![inverse.clone(), inverse];
        let buffer = Buffer::new_from_vec(
            &vulkan_context,
            &joint_matrices,
            ash::vk::BufferUsageFlags::STORAGE_BUFFER,
        )
        .unwrap();

        let skin = Skin {
            joint_matrices,
            buffer,
        };

        // Now create the skin entity
        let skinned_entity = world.push((skin, root_transform));

        // Create a child joint
        let child_joint = Joint {
            skeleton_root: skinned_entity,
            inverse_bind_matrix: Matrix4::identity(),
        };

        let mut child_transform = Transform::default();
        child_transform.translation = vec3(1.0, 0.0, 0.0);
        child_transform.parent = Some(skinned_entity);
        child_transform.local_matrix = get_local_matrix(&child_transform);
        child_transform.global_matrix = get_global_matrix(&child_transform, &world);
        println!("child_transform: {:?}", child_transform);
        let child = world.push((child_joint, child_transform));

        // Create a grandchild joint
        let grandchild_joint = Joint {
            skeleton_root: skinned_entity,
            inverse_bind_matrix: Matrix4::identity(),
        };

        let mut grandchild_transform = Transform::default();
        grandchild_transform.translation = vec3(0.0, 1.0, 0.0);
        grandchild_transform.parent = Some(child);
        grandchild_transform.local_matrix = get_local_matrix(&grandchild_transform);
        grandchild_transform.global_matrix = get_global_matrix(&grandchild_transform, &world);
        let _grandchild = world.push((grandchild_joint, grandchild_transform));

        let mut schedule = Schedule::builder().add_system(skinning_system()).build();
        let mut resources = Resources::default();
        resources.insert(vulkan_context.clone());
        schedule.execute(&mut world, &mut resources);

        let skin = world.entry(skinned_entity).unwrap();
        let skin = skin.get_component::<Skin>().unwrap();

        let matrices_from_buffer: &[Matrix4<f32>];

        unsafe {
            let memory = vulkan_context
                .device
                .map_memory(
                    skin.buffer.device_memory,
                    0,
                    ash::vk::WHOLE_SIZE,
                    ash::vk::MemoryMapFlags::empty(),
                )
                .unwrap();
            matrices_from_buffer = std::slice::from_raw_parts_mut(std::mem::transmute(memory), 2);
        }

        assert_eq!(matrices_from_buffer.len(), 2);
        for (from_buf, joint_matrices) in
            matrices_from_buffer.iter().zip(skin.joint_matrices.iter())
        {
            assert_ne!(*from_buf, Matrix4::identity());
            assert_ne!(from_buf, joint_matrices);
        }
    }
}
