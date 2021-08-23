use cgmath::{Matrix4, SquareMatrix};
use legion::{system, world::SubWorld, Entity, EntityStore, IntoQuery};
use std::collections::HashMap;

use crate::{
    components::{Info, Joint, Mesh, Skin, TransformMatrix},
    resources::VulkanContext,
};

#[system]
#[read_component(Joint)]
#[read_component(TransformMatrix)]
#[read_component(Mesh)]
#[read_component(Info)]
#[read_component(Skin)]
pub(crate) fn skinning(world: &mut SubWorld, #[resource] vulkan_context: &VulkanContext) -> () {
    let mut joint_matrices: HashMap<Entity, HashMap<usize, Matrix4<f32>>> = HashMap::new();
    unsafe {
        let mut query = <(&TransformMatrix, &Joint, &Info)>::query();
        query.for_each_unchecked(world, |(transform_matrix, joint, info)| {
            let skeleton_root = joint.skeleton_root;
            let skeleton_root_entity = world.entry_ref(skeleton_root).unwrap();
            let inverse_transform = skeleton_root_entity
                .get_component::<TransformMatrix>()
                .unwrap()
                .0
                .invert()
                .unwrap();
            let joint_transform = transform_matrix.0;
            let inverse_bind_matrix = joint.inverse_bind_matrix;
            let id = info.node_id;

            let joint_matrix = inverse_transform * joint_transform * inverse_bind_matrix;
            let matrices = joint_matrices.entry(skeleton_root).or_default();
            matrices.insert(id, joint_matrix);
        });
    }

    let mut query = <(&Mesh, &Skin)>::query();
    query.for_each_chunk(world, |chunk| {
        for (entity, (mesh, skin)) in chunk.into_iter_entities() {
            let mut matrices_map = joint_matrices.remove(&entity).expect(&format!(
                "Unable to get joint_matrix for entity: {:?}",
                entity
            ));
            let buffer = &mesh.storage_buffer;
            let mut matrices = Vec::new();
            for joint_id in &skin.joint_ids {
                let joint_matrix = matrices_map.remove(&joint_id).unwrap();
                matrices.push(joint_matrix);
            }
            buffer
                .update(vulkan_context, matrices.as_ptr(), matrices.len())
                .unwrap();
        }
    });
}

#[cfg(test)]
mod tests {

    use std::{io::Write, marker::PhantomData};

    use crate::{
        add_model_to_world,
        buffer::Buffer,
        components::{Joint, Parent, Skin},
        gltf_loader::load_models_from_gltf,
        resources::{render_context::create_descriptor_set_layouts, VulkanContext},
        systems::skinning_system,
        Vertex,
    };

    use super::*;
    use ash::{version::DeviceV1_0, vk};
    use cgmath::{relative_eq, vec3, Matrix4, SquareMatrix};
    use legion::{component, Resources, Schedule, World};

    #[test]
    pub fn test_skinning_system() {
        let mut world = World::default();
        let vulkan_context = VulkanContext::testing().unwrap();

        // Create the transform for the skin entity
        let translation = vec3(1.0, 2.0, 3.0);
        let root_transform_matrix = Matrix4::from_translation(translation);

        // Create a skin
        let inverse = root_transform_matrix.invert().unwrap();
        let joint_matrices = vec![inverse.clone(), inverse];
        let storage_buffer = Buffer::new_from_vec(
            &vulkan_context,
            &joint_matrices,
            ash::vk::BufferUsageFlags::STORAGE_BUFFER,
        )
        .unwrap();

        // Create a Mesh
        let mesh = Mesh {
            descriptor_sets: [vk::DescriptorSet::null()],
            num_indices: 0,
            index_buffer: Buffer {
                handle: vk::Buffer::null(),
                device_memory: vk::DeviceMemory::null(),
                size: 0,
                _phantom: PhantomData::<u32>,
            },
            vertex_buffer: Buffer {
                handle: vk::Buffer::null(),
                device_memory: vk::DeviceMemory::null(),
                size: 0,
                _phantom: PhantomData::<Vertex>,
            },
            storage_buffer,
        };

        // Now create the skin entity
        let skinned_entity = world.push((
            mesh,
            TransformMatrix(root_transform_matrix),
            Skin {
                joint_ids: vec![0, 1],
            },
        ));

        // Create a child joint
        let child_joint = Joint {
            skeleton_root: skinned_entity,
            inverse_bind_matrix: Matrix4::identity(),
        };

        let child_translation = vec3(1.0, 0.0, 0.0);
        let matrix = Matrix4::from_translation(child_translation);
        let child = world.push((
            child_joint,
            TransformMatrix(matrix),
            Parent(skinned_entity),
            Info {
                name: "0".to_string(),
                node_id: 0,
            },
        ));

        // Create a grandchild joint
        let grandchild_joint = Joint {
            skeleton_root: skinned_entity,
            inverse_bind_matrix: Matrix4::identity(),
        };

        let grandchild_translation = vec3(1.0, 0.0, 0.0);
        let matrix = Matrix4::from_translation(grandchild_translation);
        let _grandchild = world.push((
            grandchild_joint,
            TransformMatrix(matrix),
            Parent(child),
            Info {
                name: "1".to_string(),
                node_id: 1,
            },
        ));

        let mut schedule = Schedule::builder().add_system(skinning_system()).build();
        let mut resources = Resources::default();
        resources.insert(vulkan_context.clone());
        schedule.execute(&mut world, &mut resources);

        let entry = world.entry(skinned_entity).unwrap();
        let mesh = entry.get_component::<Mesh>().unwrap();

        let matrices_from_buffer: &[Matrix4<f32>];

        unsafe {
            let memory = vulkan_context
                .device
                .map_memory(
                    mesh.storage_buffer.device_memory,
                    0,
                    ash::vk::WHOLE_SIZE,
                    ash::vk::MemoryMapFlags::empty(),
                )
                .unwrap();
            matrices_from_buffer = std::slice::from_raw_parts_mut(std::mem::transmute(memory), 2);
        }

        assert_eq!(matrices_from_buffer.len(), 2);
        for (from_buf, joint_matrices) in matrices_from_buffer.iter().zip(joint_matrices.iter()) {
            assert_ne!(*from_buf, Matrix4::identity());
            assert_ne!(from_buf, joint_matrices);
        }
    }

    #[test]
    pub fn test_hand_skinning() {
        let vulkan_context = VulkanContext::testing().unwrap();
        let set_layouts = create_descriptor_set_layouts(&vulkan_context).unwrap();

        let data: Vec<(&[u8], &[u8])> = vec![
            (
                include_bytes!("../../../hotham-asteroid/assets/left_hand.gltf"),
                include_bytes!("../../../hotham-asteroid/assets/left_hand.bin"),
            ),
            (
                include_bytes!("../../../hotham-asteroid/assets/right_hand.gltf"),
                include_bytes!("../../../hotham-asteroid/assets/right_hand.bin"),
            ),
        ];
        let models = load_models_from_gltf(data, &vulkan_context, set_layouts.mesh_layout).unwrap();

        let mut world = World::default();

        // Add two hands
        let _left_hand = add_model_to_world("Left Hand", &models, &mut world, None).unwrap();
        let _right_hand = add_model_to_world("Right Hand", &models, &mut world, None).unwrap();

        let mut resources = Resources::default();
        resources.insert(vulkan_context);

        let mut schedule = Schedule::builder().add_system(skinning_system()).build();
        schedule.execute(&mut world, &mut resources);

        let mut query = Entity::query().filter(component::<Skin>() & component::<Mesh>());
        let mut called = 0;
        unsafe {
            query.for_each_unchecked(&world, |skinned_entity| {
                let entry = world.entry_ref(*skinned_entity).unwrap();
                let mesh = entry.get_component::<Mesh>().unwrap();
                let info = entry.get_component::<Info>().unwrap();
                let storage_buffer = &mesh.storage_buffer;

                let vulkan_context = resources.get::<VulkanContext>().unwrap();
                let memory = vulkan_context
                    .device
                    .map_memory(
                        storage_buffer.device_memory,
                        0,
                        ash::vk::WHOLE_SIZE,
                        ash::vk::MemoryMapFlags::empty(),
                    )
                    .unwrap();
                let matrices_from_buffer: &[Matrix4<f32>];
                let size = storage_buffer.size;
                let size_of_matrix = std::mem::size_of::<Matrix4<f32>>() as u64;
                let len = size / size_of_matrix;
                assert_eq!(len, 25);
                matrices_from_buffer =
                    std::slice::from_raw_parts_mut(std::mem::transmute(memory), len as _);

                assert_eq!(matrices_from_buffer.len(), 25);
                let correct_matrices: Vec<Matrix4<f32>> = if info.name == "hands:Lhand" {
                    println!("Left hand!");
                    serde_json::from_slice(include_bytes!(
                        "../../../test_assets/left_hand_skinned_matrices.json"
                    ))
                    .unwrap()
                } else {
                    println!("Right hand!");
                    serde_json::from_slice(include_bytes!(
                        "../../../test_assets/right_hand_skinned_matrices.json"
                    ))
                    .unwrap()
                };
                for i in 0..matrices_from_buffer.len() {
                    let expected = correct_matrices[i];
                    let actual = matrices_from_buffer[i];
                    if !relative_eq!(expected, actual) {
                        println!("Matrix {} is incorrect", i);
                        println!("Actual:");
                        println!("{}", serde_json::to_string_pretty(&actual).unwrap());
                        println!("Expected:");
                        println!("{}", serde_json::to_string_pretty(&expected).unwrap());
                        std::fs::File::create("matrix_failed.json")
                            .unwrap()
                            .write_all(&serde_json::to_vec_pretty(&matrices_from_buffer).unwrap())
                            .unwrap();
                        panic!("FAIL!");
                    }
                }
                called += 1;
            });
        }

        assert_ne!(called, 0);
    }
}
