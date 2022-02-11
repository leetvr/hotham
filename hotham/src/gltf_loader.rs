use crate::{
    buffer::Buffer,
    components::{
        animation_controller::AnimationController, AnimationTarget, Info, Joint, Mesh, Parent,
        Root, Skin, Transform, TransformMatrix, Visible,
    },
    resources::{render_context::DescriptorSetLayouts, VulkanContext},
};
use anyhow::Result;
use ash::vk;
use gltf::animation::util::ReadOutputs;
use hecs::{Entity, World};
use itertools::{izip, Itertools};
use nalgebra::{vector, Matrix4, Quaternion, UnitQuaternion};
use std::collections::HashMap;

/// Convenience type for models
pub type Models = HashMap<String, World>;

/// Load glTF models from a GLB file
pub fn load_models_from_glb(
    glb_bufs: &Vec<&[u8]>,
    vulkan_context: &VulkanContext,
    descriptor_set_layouts: &DescriptorSetLayouts,
) -> Result<Models> {
    let mut models = HashMap::new();

    for glb_buf in glb_bufs {
        let (document, buffers, images) = gltf::import_slice(glb_buf).unwrap();
        load_models_from_gltf_data(
            &document,
            &buffers[0],
            &images,
            &vulkan_context,
            descriptor_set_layouts,
            &mut models,
        )
        .unwrap();
    }

    Ok(models)
}

/// Load glTF models from a glTF document
pub fn load_models_from_gltf_data(
    document: &gltf::Document,
    buffer: &[u8],
    images: &Vec<gltf::image::Data>,
    vulkan_context: &VulkanContext,
    descriptor_set_layouts: &DescriptorSetLayouts,
    models: &mut Models,
) -> Result<()> {
    let root_scene = document.scenes().next().unwrap(); // safe as there is always one scene
    let mut node_entity_map = HashMap::new();
    let animations = document.animations().collect_vec();

    for node_data in root_scene.nodes() {
        let mut world = World::default();
        load_node(
            &node_data,
            buffer,
            vulkan_context,
            descriptor_set_layouts,
            &mut world,
            &mut node_entity_map,
            true,
            images,
        )?;
        add_parents(&node_data, &mut world, &mut node_entity_map);
        add_skins_and_joints(
            &node_data,
            buffer,
            &mut world,
            &vulkan_context,
            &mut node_entity_map,
        );
        add_animations(&animations, &buffer, &mut world, &mut node_entity_map);

        models.insert(
            node_data.name().expect("Node has no name!").to_string(),
            world,
        );
    }

    Ok(())
}

fn load_node(
    node_data: &gltf::Node,
    gltf_buffer: &[u8],
    vulkan_context: &VulkanContext,
    descriptor_set_layouts: &DescriptorSetLayouts,
    world: &mut World,
    node_entity_map: &mut HashMap<usize, Entity>,
    is_root: bool,
    images: &Vec<gltf::image::Data>,
) -> Result<()> {
    let transform = Transform::load(node_data.transform());
    let transform_matrix = TransformMatrix(node_data.transform().matrix().into());
    let info = Info {
        name: node_data
            .name()
            .map(|s| s.to_string())
            .unwrap_or(format!("Node {}", node_data.index())),
        node_id: node_data.index(),
    };
    let this_entity = world.spawn((transform, transform_matrix, info));
    node_entity_map.insert(node_data.index(), this_entity);

    if let Some(mesh) = node_data.mesh() {
        let mesh = Mesh::load(
            &mesh,
            gltf_buffer,
            vulkan_context,
            descriptor_set_layouts,
            images,
        )?;

        world.insert(this_entity, (mesh, Visible {})).unwrap();
    }

    if is_root {
        world.insert_one(this_entity, Root {}).unwrap();
    }

    for child in node_data.children() {
        load_node(
            &child,
            gltf_buffer,
            vulkan_context,
            descriptor_set_layouts,
            world,
            node_entity_map,
            false,
            images,
        )?;
    }

    Ok(())
}

fn add_parents(
    node_data: &gltf::Node,
    world: &mut World,
    node_entity_map: &mut HashMap<usize, Entity>,
) -> () {
    let this_entity = node_entity_map.get(&node_data.index()).unwrap();
    let parent = Parent(*this_entity);
    for child_node in node_data.children() {
        let child_id = child_node.index();
        let child_entity = node_entity_map.get(&child_id).unwrap();
        world.insert_one(*child_entity, parent.clone()).unwrap();
        add_parents(&child_node, world, node_entity_map);
    }
}

fn add_skins_and_joints(
    node_data: &gltf::Node,
    buffer: &[u8],
    world: &mut World,
    vulkan_context: &VulkanContext,
    node_entity_map: &mut HashMap<usize, Entity>,
) -> () {
    // Do we need to add a Skin?
    // TODO: Extract this to components::Skin
    if let Some(node_skin_data) = node_data.skin() {
        println!("[HOTHAM_GLTF] Adding a skin to {}", node_data.index());
        let this_entity = *node_entity_map.get(&node_data.index()).unwrap();
        let mut joint_matrices = Vec::new();
        let reader = node_skin_data.reader(|_| Some(buffer));
        let matrices = reader.read_inverse_bind_matrices().unwrap();
        for m in matrices {
            let m = Matrix4::from(m);
            joint_matrices.push(m);
        }
        let mut joint_ids = Vec::new();

        for (joint_node, inverse_bind_matrix) in node_skin_data.joints().zip(joint_matrices.iter())
        {
            let joint = Joint {
                skeleton_root: this_entity,
                inverse_bind_matrix: inverse_bind_matrix.clone(),
            };
            joint_ids.push(joint_node.index());
            let joint_entity = node_entity_map.get(&joint_node.index()).unwrap();
            world.insert_one(*joint_entity, joint).unwrap();
        }

        // Add a Skin to the entity.
        world.insert_one(this_entity, Skin { joint_ids }).unwrap();

        // Tell the vertex shader how many joints we have
        let mut mesh = world.get_mut::<Mesh>(this_entity).unwrap();
        mesh.ubo_data.joint_count = joint_matrices.len() as f32;
    }

    for child in node_data.children() {
        add_skins_and_joints(&child, buffer, world, vulkan_context, node_entity_map);
    }
}

fn add_animations(
    animations: &Vec<gltf::Animation>,
    buffer: &[u8],
    world: &mut World,
    node_entity_map: &mut HashMap<usize, Entity>,
) -> () {
    let (controller_entity, _) = world.query::<&Root>().iter().next().unwrap();

    for animation in animations.iter() {
        'chunks: for chunk in &animation.channels().chunks(3) {
            let mut translations = Vec::new();
            let mut rotations = Vec::new();
            let mut scales = Vec::new();
            let mut target = 0;
            '_channels: for channel in chunk {
                target = channel.target().node().index();
                if !node_entity_map.contains_key(&target) {
                    continue 'chunks;
                }

                let reader = channel.reader(|_| Some(buffer));
                match reader.read_outputs() {
                    Some(ReadOutputs::Translations(translation_data)) => {
                        for t in translation_data {
                            translations.push(vector![t[0], t[1], t[2]]);
                        }
                    }
                    Some(ReadOutputs::Rotations(rotation_data)) => {
                        for r in rotation_data.into_f32() {
                            rotations.push(UnitQuaternion::new_normalize(Quaternion::new(
                                r[3], r[0], r[1], r[2],
                            )));
                            // gltf gives us a quaternion in [x, y, z, w] but we need [w, x, y, z]
                        }
                    }
                    Some(ReadOutputs::Scales(scale_data)) => {
                        for s in scale_data {
                            scales.push(vector![s[0], s[1], s[2]]);
                        }
                    }
                    _ => {}
                }
            }

            let target_entity = *node_entity_map.get(&target).unwrap();
            if !world.contains(target_entity) {
                println!("[HOTHAM_GLTF] - Error importing animation {:?}. No target, probably due to malformed file. Ignoring", animation.name());
                return;
            }

            assert!(
                translations.len() == rotations.len() && rotations.len() == scales.len(),
                "Animation {} - {:?} has malformed data for node {}. translations.len() - {}, rotations.len() - {}, scales.len() - {}",
                animation.index(),
                animation.name(),
                target,
                translations.len(),
                rotations.len(),
                scales.len(),
            );

            let animation = izip!(translations, rotations, scales)
                .map(|(t, r, s)| Transform {
                    translation: t,
                    rotation: r,
                    scale: s,
                })
                .collect_vec();

            // If an animation target exists already, push this animation onto it. Otherwise, create a new one.
            match world.query_one_mut::<&mut AnimationTarget>(target_entity) {
                Ok(animation_target) => {
                    animation_target.animations.push(animation);
                }
                _ => {
                    world
                        .insert_one(
                            target_entity,
                            AnimationTarget {
                                controller: controller_entity.clone(),
                                animations: vec![animation],
                            },
                        )
                        .unwrap();
                }
            }

            // Add an animation controller to our parent, if needed.
            let entity_ref = world.entity(controller_entity).unwrap();
            if !entity_ref.has::<AnimationController>() {
                world
                    .insert_one(controller_entity, AnimationController::default())
                    .unwrap();
            }
        }
    }
}

/// Convenience function to add a glTF model to the world referenced by its node name
pub fn add_model_to_world(
    name: &str,
    models: &Models,
    destination_world: &mut World,
    parent: Option<Entity>,
    vulkan_context: &VulkanContext,
    descriptor_set_layouts: &DescriptorSetLayouts,
) -> Option<Entity> {
    let source_world = models.get(name)?;
    let source_entities = source_world.iter();
    let mut entity_map = HashMap::new();

    // Reserve some empty entities in the new world for us to use.
    let new_entities = destination_world.reserve_entities(source_entities.len() as _);

    // Create a map from the source entity to the new destination entity.
    for (source_entity, destination_entity) in source_entities.zip(new_entities) {
        let source_entity = source_entity.entity();
        entity_map.insert(source_entity, destination_entity);
    }

    // Go through each entity in the source world and clone its components into the new world.
    for (source_entity, destination_entity) in &entity_map {
        if let Ok(transform) = source_world.get_mut::<Transform>(*source_entity) {
            destination_world
                .insert_one(*destination_entity, transform.clone())
                .unwrap();
        }

        if let Ok(transform_matrix) = source_world.get_mut::<TransformMatrix>(*source_entity) {
            destination_world
                .insert_one(*destination_entity, transform_matrix.clone())
                .unwrap();
        }

        // Create a new mesh for this entity in the destination world.
        if let Ok(mesh) = source_world.get_mut::<Mesh>(*source_entity) {
            let info = source_world.get_mut::<Info>(*source_entity).unwrap();

            // Create new description sets
            let descriptor_sets = vulkan_context
                .create_mesh_descriptor_sets(descriptor_set_layouts.mesh_layout, &info.name)
                .unwrap();

            // Create a new buffer
            let ubo_buffer = Buffer::new(
                vulkan_context,
                &[mesh.ubo_data],
                vk::BufferUsageFlags::UNIFORM_BUFFER,
            )
            .unwrap();
            vulkan_context.update_buffer_descriptor_set(
                &ubo_buffer,
                mesh.descriptor_sets[0],
                0,
                vk::DescriptorType::UNIFORM_BUFFER,
            );

            let new_mesh = Mesh {
                descriptor_sets: [descriptor_sets[0]],
                ubo_buffer,
                ubo_data: mesh.ubo_data.clone(),
                primitives: mesh.primitives.clone(),
            };
            destination_world
                .insert_one(*destination_entity, new_mesh)
                .unwrap();
        }

        if let Ok(skin) = source_world.get_mut::<Skin>(*source_entity) {
            destination_world
                .insert_one(*destination_entity, skin.clone())
                .unwrap();
        }

        // If the source entity had a joint, clone it and set the skeleton root to the corresponding entity in the destination world.
        if let Ok(joint) = source_world.get_mut::<Joint>(*source_entity) {
            let mut new_joint = joint.clone();
            new_joint.skeleton_root = *entity_map.get(&joint.skeleton_root).unwrap();
            destination_world
                .insert_one(*destination_entity, new_joint)
                .unwrap();
        }

        // If the source entity had a parent, set it to the corresponding entity in the destination world.
        if let Ok(parent) = source_world.get_mut::<Parent>(*source_entity) {
            let new_parent = entity_map.get(&parent.0).unwrap();
            destination_world
                .insert_one(*destination_entity, Parent(*new_parent))
                .unwrap();
        }

        if let Ok(root) = source_world.get_mut::<Root>(*source_entity) {
            destination_world
                .insert_one(*destination_entity, root.clone())
                .unwrap();

            // Set a parent for the root entity if one was specified.
            if let Some(parent) = parent {
                destination_world
                    .insert_one(*destination_entity, Parent(parent))
                    .unwrap();
            }
        }

        if let Ok(info) = source_world.get_mut::<Info>(*source_entity) {
            destination_world
                .insert_one(*destination_entity, info.clone())
                .unwrap();
        }

        if let Ok(animation_controller) =
            source_world.get_mut::<AnimationController>(*source_entity)
        {
            destination_world
                .insert_one(*destination_entity, animation_controller.clone())
                .unwrap();
        }

        if let Ok(animation_target) = source_world.get_mut::<AnimationTarget>(*source_entity) {
            let mut new_animation_target = animation_target.clone();
            new_animation_target.controller =
                *entity_map.get(&animation_target.controller).unwrap();
            destination_world
                .insert_one(*destination_entity, new_animation_target)
                .unwrap();
        }

        if let Ok(visible) = source_world.get_mut::<Visible>(*source_entity) {
            destination_world
                .insert_one(*destination_entity, visible.clone())
                .unwrap();
        }
    }

    // Find the root entity of the source world.
    let (root_entity, _) = source_world.query::<&Root>().iter().next().unwrap();

    // Get the new root entity.
    let new_root_entity = entity_map.get(&root_entity).cloned().unwrap();

    // We'll also need to fix up any meshes
    for (_, (info, mesh)) in destination_world.query_mut::<(&Info, &mut Mesh)>() {
        // Create new description sets
        let new_descriptor_sets = vulkan_context
            .create_mesh_descriptor_sets(descriptor_set_layouts.mesh_layout, &info.name)
            .unwrap();
        mesh.descriptor_sets = [new_descriptor_sets[0]];

        // Create a new buffer
        let new_ubo_buffer = Buffer::new(
            vulkan_context,
            &[mesh.ubo_data],
            vk::BufferUsageFlags::UNIFORM_BUFFER,
        )
        .unwrap();
        vulkan_context.update_buffer_descriptor_set(
            &new_ubo_buffer,
            mesh.descriptor_sets[0],
            0,
            vk::DescriptorType::UNIFORM_BUFFER,
        );
        mesh.ubo_buffer = new_ubo_buffer;
    }

    Some(new_root_entity)
}

#[cfg(target_os = "windows")]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        components::{Root, Transform},
        resources::{render_context::create_descriptor_set_layouts, VulkanContext},
    };
    use approx::assert_relative_eq;

    #[test]
    pub fn test_load_models() {
        let vulkan_context = VulkanContext::testing().unwrap();
        let set_layouts = create_descriptor_set_layouts(&vulkan_context).unwrap();

        let data: Vec<&[u8]> = vec![
            include_bytes!("../../test_assets/damaged_helmet.glb"),
            include_bytes!("../../test_assets/asteroid.glb"),
        ];
        let models = load_models_from_glb(&data, &vulkan_context, &set_layouts).unwrap();
        let test_data = vec![
            (
                "Asteroid",
                0,
                vector![0., 0., 0.],
                Quaternion::new(1., 0., 0., 0.),
            ),
            (
                "Refinery",
                1,
                vector![-0.06670809, 2.1408155, -0.46151406],
                Quaternion::new(
                    0.719318151473999,
                    -0.09325116872787476,
                    0.6883626580238342,
                    0.006518156733363867,
                ),
            ),
            (
                "Damaged Helmet",
                0,
                vector![0., 1.4, 0.],
                Quaternion::new(0.707, 0.707, 0., 0.),
            ),
        ];
        for (name, id, translation, rotation) in &test_data {
            let model_world = models
                .get(*name)
                .expect(&format!("Unable to find model with name {}", name));

            let mut world = World::default();
            let model = add_model_to_world(
                *name,
                &models,
                &mut world,
                None,
                &vulkan_context,
                &set_layouts,
            );
            assert!(model.is_some(), "Model {} could not be added", name);

            let model = model.unwrap();
            let (info, transform, mesh, ..) = world
                .query_one_mut::<(&Info, &Transform, &Mesh, &TransformMatrix, &Root)>(model)
                .unwrap();
            assert_eq!(
                transform.translation, *translation,
                "Model {} has wrong translation",
                name
            );
            assert_eq!(
                transform.rotation,
                UnitQuaternion::new_normalize(*rotation),
                "Model {} has wrong rotation",
                name
            );
            assert_eq!(&info.name, *name);
            assert_eq!(&info.node_id, id, "Node {} has wrong ID", name);

            // Get mesh's initial buffer handle
            let mut original_mesh =
                model_world.query::<(&Mesh, &Transform, &TransformMatrix, &Info)>();
            let original_mesh = original_mesh.iter().next().unwrap().1 .0;
            let initial_buffer = original_mesh.ubo_buffer.handle;
            let new_buffer = mesh.ubo_buffer.handle;
            assert_ne!(initial_buffer, new_buffer);
        }
    }

    #[test]
    pub fn test_hand() {
        let vulkan_context = VulkanContext::testing().unwrap();
        let set_layouts = create_descriptor_set_layouts(&vulkan_context).unwrap();

        let data: Vec<&[u8]> = vec![include_bytes!("../../test_assets/left_hand.glb")];
        let models = load_models_from_glb(&data, &vulkan_context, &set_layouts).unwrap();

        let mut world = World::default();
        let _hand = add_model_to_world(
            "Left Hand",
            &models,
            &mut world,
            None,
            &vulkan_context,
            &set_layouts,
        );

        // Make sure there is only one root
        let mut roots = world.query_mut::<(&Root, &Info, &Transform)>().into_iter();
        assert_eq!(roots.len(), 1);
        let root = roots.next().unwrap().1;
        assert_eq!(&root.1.name, "Left Hand");

        // Make sure its transform is correct
        assert_relative_eq!(root.2.translation, vector![0.0, 0.0, 0.0]);

        // Make sure we imported the mesh
        let meshes = world
            .query_mut::<(&Mesh, &Transform, &TransformMatrix)>()
            .into_iter();
        assert_eq!(meshes.len(), 1);

        // Make sure we imported the AnimationController
        let animation_controllers = world.query_mut::<&AnimationController>().into_iter();
        assert_eq!(animation_controllers.len(), 1);

        // Make sure we got all the nodes
        let transforms = world.query_mut::<&Transform>().into_iter();
        assert_eq!(transforms.len(), 28);

        // Make sure we got all the Parent -> Child relationships
        {
            let mut transforms_with_parents = world.query::<(&Transform, &Parent)>();
            assert_eq!(transforms_with_parents.iter().len(), 27);
            for (_, (_, parent)) in transforms_with_parents.iter() {
                assert!(world.contains(parent.0));
            }
        }

        // Make sure we got all the joints
        {
            let mut joints = world.query::<&Joint>();
            assert_eq!(joints.iter().len(), 25);
            for (_, joint) in joints.iter() {
                assert!(world.contains(joint.skeleton_root));
            }
        }

        // Make sure we got all the AnimationTargets
        {
            let mut animation_targets = world.query::<&AnimationTarget>();
            assert_eq!(animation_targets.iter().len(), 17);
            for (_, animation_target) in animation_targets.iter() {
                assert!(world.contains(animation_target.controller));
            }
        }
    }
}
