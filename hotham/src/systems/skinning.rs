use hecs::{Entity, PreparedQuery, World};
use nalgebra::Matrix4;
use std::collections::HashMap;

use crate::components::{Info, Joint, Mesh, Skin, TransformMatrix};

/// Skinning system
/// Walks through each joint in the system and builds up the `joint_matrices` that will be sent to the vertex shader
pub fn skinning_system(
    joints_query: &mut PreparedQuery<(&TransformMatrix, &Joint, &Info)>,
    meshes_query: &mut PreparedQuery<(&mut Mesh, &Skin)>,
    world: &mut World,
) {
    let mut joint_matrices: HashMap<Entity, HashMap<usize, Matrix4<f32>>> = HashMap::new();
    for (_, (transform_matrix, joint, info)) in joints_query.query(world).iter() {
        let inverse_transform = world
            .get_mut::<TransformMatrix>(joint.skeleton_root)
            .unwrap()
            .0
            .try_inverse()
            .unwrap();
        let joint_transform = transform_matrix.0;
        let inverse_bind_matrix = joint.inverse_bind_matrix;
        let id = info.node_id;

        let joint_matrix = inverse_transform * joint_transform * inverse_bind_matrix;
        let matrices = joint_matrices.entry(joint.skeleton_root).or_default();
        matrices.insert(id, joint_matrix);
    }

    for (entity, (mesh, skin)) in meshes_query.query_mut(world) {
        let mut matrices_map = joint_matrices
            .remove(&entity)
            .unwrap_or_else(|| panic!("Unable to get joint_matrix for entity: {:?}", entity));
        let joint_matrices = &mut mesh.ubo_data.joint_matrices;
        for (i, joint_id) in skin.joint_ids.iter().enumerate() {
            let joint_matrix = matrices_map.remove(joint_id).unwrap();
            joint_matrices[i] = joint_matrix;
        }
    }
}

#[cfg(target_os = "windows")]
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
    use hecs::Satisfies;
    use nalgebra::vector;

    #[test]
    pub fn test_skinning_system() {
        let mut world = World::new();

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
        };

        // Now create the skin entity
        let skinned_entity = world.spawn((
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
        let child = world.spawn((
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
        let _grandchild = world.spawn((
            grandchild_joint,
            TransformMatrix(matrix),
            Parent(child),
            Info {
                name: "1".to_string(),
                node_id: 1,
            },
        ));

        skinning_system(&mut Default::default(), &mut Default::default(), &mut world);

        let mesh = world.get_mut::<Mesh>(skinned_entity).unwrap();
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

        skinning_system(&mut Default::default(), &mut Default::default(), &mut world);
        for (_, mesh) in world.query_mut::<&Mesh>() {
            mesh.ubo_buffer
                .update(&vulkan_context, &[mesh.ubo_data])
                .unwrap();
        }

        let mut called = 0;
        for skinned_entity in world
            .query::<Satisfies<(&Skin, &Mesh)>>()
            .iter()
            .filter_map(|(e, s)| if s { Some(e) } else { None })
        {
            let mut query = world.query_one::<(&Mesh, &Info)>(skinned_entity).unwrap();
            let (mesh, info) = query.get().unwrap();
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
            let ubo = unsafe { get_from_device_memory(&vulkan_context, &mesh.ubo_buffer) };
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
        }

        assert_ne!(called, 0);
    }
}
