use crate::{
    buffer::Buffer,
    components::{joint, skin, Joint, Mesh, Parent, Skin, Transform, TransformMatrix},
    resources::VulkanContext,
};
use anyhow::Result;
use ash::vk;
use cgmath::{Matrix4, SquareMatrix};
use itertools::Itertools;
use legion::{Entity, EntityStore, World};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    io::Cursor,
    rc::{Rc, Weak},
};

struct SkinData {
    skeleton_root: Entity,
    inverse_bind_matrices: HashMap<usize, Matrix4<f32>>,
}

pub(crate) fn load_models_from_gltf(
    gltf_bytes: &[u8],
    buffers: &Vec<&[u8]>,
    vulkan_context: &VulkanContext,
    mesh_descriptor_set_layout: vk::DescriptorSetLayout,
) -> Result<HashMap<String, World>> {
    let mut models = HashMap::new();
    let gtlf_buf = Cursor::new(gltf_bytes);
    let gltf = gltf::Gltf::from_reader(gtlf_buf)?;
    let document = gltf.document;
    let blob = buffers;

    let root_scene = document.scenes().next().unwrap(); // safe as there is always one scene

    for node_data in root_scene.nodes() {
        let mut world = World::default();
        load_node(
            &node_data,
            blob,
            vulkan_context,
            mesh_descriptor_set_layout,
            &mut world,
            None,
            &None,
        )?;
        models.insert(
            node_data.name().expect("Node has no name!").to_string(),
            world,
        );
    }

    for animation in document.animations() {
        // TODO
        // Animation::load(&animation, blob, &nodes_vec)?;
    }

    Ok(models)
}

fn load_node(
    node_data: &gltf::Node,
    gltf_buffers: &Vec<&[u8]>,
    vulkan_context: &VulkanContext,
    mesh_descriptor_set_layout: vk::DescriptorSetLayout,
    world: &mut World,
    parent: Option<Parent>,
    skin_data: &Option<SkinData>,
) -> Result<()> {
    let transform = Transform::load(node_data.transform());
    let transform_matrix = TransformMatrix(node_data.transform().matrix().into());
    let this_entity = world.push((transform, transform_matrix));
    let mut e = world.entry(this_entity).unwrap();

    if let Some(mesh) = node_data.mesh() {
        let empty_matrix: Matrix4<f32> = Matrix4::identity();
        let empty_skin_buffer = Buffer::new(
            &vulkan_context,
            &empty_matrix,
            vk::BufferUsageFlags::STORAGE_BUFFER,
        )?;
        let mesh = Mesh::load(
            &mesh,
            gltf_buffers,
            vulkan_context,
            mesh_descriptor_set_layout,
            &empty_skin_buffer,
        )?;

        e.add_component(mesh);
    }

    // Do we need to add a Parent?
    if let Some(parent) = parent {
        e.add_component(parent);
    }

    // Do we need to add a joint?
    if let Some(skin_data) = skin_data {
        if let Some(inverse_bind_matrix) = skin_data.inverse_bind_matrices.get(&node_data.index()) {
            let joint = Joint {
                skeleton_root: skin_data.skeleton_root,
                inverse_bind_matrix: inverse_bind_matrix.clone(),
            };
            e.add_component(joint);
        }
    }

    // Do we need to add a Skin?
    let skin_data = if let Some(node_skin_data) = node_data.skin() {
        let mut joint_matrices = Vec::new();
        let reader = node_skin_data.reader(|buffer| Some(&gltf_buffers[buffer.index()]));
        let matrices = reader.read_inverse_bind_matrices().unwrap();
        for m in matrices {
            let m = Matrix4::from(m);
            joint_matrices.push(m);
        }
        let buffer = Buffer::new_from_vec(
            vulkan_context,
            &joint_matrices,
            vk::BufferUsageFlags::STORAGE_BUFFER,
        )?;

        let mut inverse_bind_matrices = HashMap::new();
        for (joint, inverse_bind_matrix) in node_skin_data.joints().zip(joint_matrices.iter()) {
            inverse_bind_matrices.insert(joint.index(), inverse_bind_matrix.clone());
        }

        let skin = Skin {
            joint_matrices,
            buffer,
        };

        e.add_component(skin);

        Some(SkinData {
            skeleton_root: this_entity,
            inverse_bind_matrices,
        })
    } else {
        None
    };

    for child in node_data.children() {
        load_node(
            &child,
            gltf_buffers,
            vulkan_context,
            mesh_descriptor_set_layout,
            world,
            Some(Parent(this_entity)),
            &skin_data,
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use ash::version::DeviceV1_0;
    use cgmath::{vec3, vec4, Matrix4, Quaternion};

    use super::*;
    use crate::{
        components::Transform,
        resources::{render_context::create_descriptor_set_layouts, VulkanContext},
        Vertex,
    };
    use legion::IntoQuery;
    #[test]
    pub fn test_asteroid() {
        let vulkan_context = VulkanContext::testing().unwrap();
        let set_layouts = create_descriptor_set_layouts(&vulkan_context).unwrap();

        let gltf = include_bytes!("../../hotham-asteroid/assets/asteroid.gltf");
        let data = include_bytes!("../../hotham-asteroid/assets/asteroid_data.bin").to_vec();
        let buffers = vec![data.as_slice()];
        let models =
            load_models_from_gltf(gltf, &buffers, &vulkan_context, set_layouts.mesh_layout)
                .unwrap();

        let asteroid = models.get("Asteroid").unwrap();
        let mut query = <(&Mesh, &Transform, &TransformMatrix)>::query();
        let meshes = query.iter(asteroid).collect::<Vec<_>>();
        assert_eq!(meshes.len(), 1);
    }

    #[test]
    pub fn test_hand() {
        let vulkan_context = VulkanContext::testing().unwrap();
        let set_layouts = create_descriptor_set_layouts(&vulkan_context).unwrap();

        let (document, buffers, _) = gltf::import("../test_assets/hand.gltf").unwrap();
        let gltf = document.into_json().to_vec().unwrap();
        let buffers = buffers.iter().map(|b| b.0.as_slice()).collect();
        let models =
            load_models_from_gltf(&gltf, &buffers, &vulkan_context, set_layouts.mesh_layout)
                .unwrap();

        let hand = models.get("Hand").unwrap();
        let mut query = <(&Mesh, &Transform, &TransformMatrix)>::query();
        let meshes = query.iter(hand).collect::<Vec<_>>();
        assert_eq!(meshes.len(), 1);

        let mut query = <&Transform>::query();
        let transforms = query.iter(hand).collect::<Vec<_>>();
        assert_eq!(transforms.len(), 18);
    }
}
