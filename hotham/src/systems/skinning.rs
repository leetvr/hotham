use legion::{system, world::SubWorld, Entity, EntityStore, IntoQuery};
use nalgebra::Matrix4;
use std::collections::HashMap;

use crate::components::{Info, Joint, Mesh, Skin, TransformMatrix};

#[system]
#[read_component(Joint)]
#[read_component(TransformMatrix)]
#[write_component(Mesh)]
#[read_component(Info)]
#[read_component(Skin)]
pub fn skinning(world: &mut SubWorld) -> () {
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
                .try_inverse()
                .unwrap();
            let joint_transform = transform_matrix.0;
            let inverse_bind_matrix = joint.inverse_bind_matrix;
            let id = info.node_id;

            let joint_matrix = inverse_transform * joint_transform * inverse_bind_matrix;
            let matrices = joint_matrices.entry(skeleton_root).or_default();
            matrices.insert(id, joint_matrix);
        });
    }

    let mut query = <(&mut Mesh, &Skin)>::query();
    query.for_each_chunk_mut(world, |chunk| {
        for (entity, (mesh, skin)) in chunk.into_iter_entities() {
            let mut matrices_map = joint_matrices.remove(&entity).expect(&format!(
                "Unable to get joint_matrix for entity: {:?}",
                entity
            ));
            let joint_matrices = &mut mesh.ubo_data.joint_matrices;
            for (i, joint_id) in skin.joint_ids.iter().enumerate() {
                let joint_matrix = matrices_map.remove(&joint_id).unwrap();
                joint_matrices[i] = joint_matrix;
            }
        }
    });
}

#[cfg(test)]
mod tests {

    use std::{io::Write, marker::PhantomData};

    use crate::{
        buffer::Buffer,
        components::{mesh::MeshUBO, Joint, Parent, Skin},
        resources::VulkanContext,
        systems::skinning_system,
        util::{get_from_device_memory, get_world_with_hands},
    };

    use super::*;
    use approx::relative_eq;
    use ash::vk;
    use legion::{component, Resources, Schedule, World};
    use nalgebra::vector;

    #[test]
    pub fn test_skinning_system() {
        let mut world = World::default();

        // Create the transform for the skin entity
        let translation = vector![1.0, 2.0, 3.0];
        let root_transform_matrix = Matrix4::new_translation(&translation);

        // Create a skin
        let inverse = root_transform_matrix.try_inverse().unwrap();
        let mut ubo_data = MeshUBO::default();
        ubo_data.joint_matrices[0] = inverse.clone();
        ubo_data.joint_matrices[1] = inverse.clone();
        ubo_data.joint_count = 2.;
        let ubo_buffer = Buffer {
            handle: vk::Buffer::null(),
            device_memory: vk::DeviceMemory::null(),
            size: 0,
            device_memory_size: 0,
            usage: vk::BufferUsageFlags::UNIFORM_BUFFER,
            _phantom: PhantomData,
        };

        // Store the invese matrices for our test
        let joint_matrices = vec![inverse.clone(), inverse];

        // Create a Mesh
        let mesh = Mesh {
            descriptor_sets: [vk::DescriptorSet::null()],
            primitives: Vec::new(),
            ubo_data,
            ubo_buffer,
            should_render: true,
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

        let child_translation = vector![1.0, 0.0, 0.0];
        let matrix = Matrix4::new_translation(&child_translation);
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

        let grandchild_translation = vector![1.0, 0.0, 0.0];
        let matrix = Matrix4::new_translation(&grandchild_translation);
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
        schedule.execute(&mut world, &mut resources);

        let entry = world.entry(skinned_entity).unwrap();
        let mesh = entry.get_component::<Mesh>().unwrap();
        let matrices_from_buffer = mesh.ubo_data.joint_matrices;

        for (from_buf, joint_matrices) in matrices_from_buffer.iter().zip(joint_matrices.iter()) {
            assert_ne!(*from_buf, Matrix4::identity());
            assert_ne!(from_buf, joint_matrices);
        }
    }

    #[test]
    pub fn test_hand_skinning() {
        let vulkan_context = VulkanContext::testing().unwrap();

        let mut world = get_world_with_hands();
        let mut resources = Resources::default();
        resources.insert(vulkan_context);

        let mut schedule = Schedule::builder()
            .add_system(skinning_system())
            .add_thread_local_fn(|world, resources| {
                let vulkan_context = resources.get::<VulkanContext>().unwrap();
                let mut query = <&Mesh>::query();
                query.for_each(world, |mesh| {
                    mesh.ubo_buffer
                        .update(&vulkan_context, &[mesh.ubo_data])
                        .unwrap();
                })
            })
            .build();
        schedule.execute(&mut world, &mut resources);

        let mut query = Entity::query().filter(component::<Skin>() & component::<Mesh>());
        let mut called = 0;
        let vulkan_context = resources.get::<VulkanContext>().unwrap();
        unsafe {
            query.for_each_unchecked(&world, |skinned_entity| {
                let entry = world.entry_ref(*skinned_entity).unwrap();
                let mesh = entry.get_component::<Mesh>().unwrap();
                let info = entry.get_component::<Info>().unwrap();
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
                let ubo = get_from_device_memory(&vulkan_context, &mesh.ubo_buffer);
                let matrices_from_buffer = ubo[0].joint_matrices.to_vec();
                for i in 0..correct_matrices.len() {
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
