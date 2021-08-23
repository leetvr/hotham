use crate::{
    buffer::Buffer,
    components::{
        animation_controller::AnimationController, AnimationTarget, Info, Joint, Mesh, Parent,
        Root, Skin, Transform, TransformMatrix,
    },
    resources::VulkanContext,
};
use anyhow::Result;
use ash::vk;
use cgmath::{vec3, Matrix4, Quaternion, SquareMatrix};
use gltf::animation::util::ReadOutputs;
use itertools::{izip, Itertools};
use legion::{any, component, world::Duplicate, Entity, IntoQuery, World};
use std::{collections::HashMap, io::Cursor};

pub(crate) fn load_models_from_gltf(
    data: Vec<(&[u8], &[u8])>,
    vulkan_context: &VulkanContext,
    mesh_descriptor_set_layout: vk::DescriptorSetLayout,
) -> Result<HashMap<String, World>> {
    let mut models = HashMap::new();

    // Create a shared, empty storage buffer for non-skin models.
    // If the model has a skin, it'll be replaced with joint matrices.
    let empty_matrix: Matrix4<f32> = Matrix4::identity();
    let vec = vec![empty_matrix];
    let empty_storage_buffer =
        Buffer::new_from_vec(&vulkan_context, &vec, vk::BufferUsageFlags::STORAGE_BUFFER)?;

    for (gltf_bytes, buffer) in &data {
        let gtlf_buf = Cursor::new(gltf_bytes);
        let gltf = gltf::Gltf::from_reader(gtlf_buf)?;
        let document = gltf.document;

        let root_scene = document.scenes().next().unwrap(); // safe as there is always one scene
        let mut node_entity_map = HashMap::new();
        let animations = document.animations().collect_vec();

        for node_data in root_scene.nodes() {
            let mut world = World::default();
            load_node(
                &node_data,
                buffer,
                vulkan_context,
                mesh_descriptor_set_layout,
                &mut world,
                &mut node_entity_map,
                &empty_storage_buffer,
                true,
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
    }

    Ok(models)
}

fn load_node(
    node_data: &gltf::Node,
    gltf_buffer: &[u8],
    vulkan_context: &VulkanContext,
    mesh_descriptor_set_layout: vk::DescriptorSetLayout,
    world: &mut World,
    node_entity_map: &mut HashMap<usize, Entity>,
    empty_storage_buffer: &Buffer<Matrix4<f32>>,
    is_root: bool,
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
    let this_entity = world.push((transform, transform_matrix, info));
    node_entity_map.insert(node_data.index(), this_entity);
    let mut e = world.entry(this_entity).unwrap();

    if let Some(mesh) = node_data.mesh() {
        let mesh = Mesh::load(
            &mesh,
            gltf_buffer,
            vulkan_context,
            mesh_descriptor_set_layout,
            empty_storage_buffer,
        )?;

        e.add_component(mesh);
    }

    if is_root {
        e.add_component(Root {});
    }

    for child in node_data.children() {
        load_node(
            &child,
            gltf_buffer,
            vulkan_context,
            mesh_descriptor_set_layout,
            world,
            node_entity_map,
            empty_storage_buffer,
            false,
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
        let mut child = world.entry(*child_entity).unwrap();
        child.add_component(parent.clone());
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
    if let Some(node_skin_data) = node_data.skin() {
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
            let mut entry = world.entry(*joint_entity).unwrap();
            entry.add_component(joint);
        }

        // Add a Skin to the entity.
        let mut entry = world.entry(this_entity).unwrap();
        entry.add_component(Skin { joint_ids });

        // Create a new storage buffer with the inverse bind matrices.
        let mesh = entry.get_component_mut::<Mesh>().unwrap();
        mesh.storage_buffer = Buffer::new_from_vec(
            vulkan_context,
            &joint_matrices,
            vk::BufferUsageFlags::STORAGE_BUFFER,
        )
        .unwrap();

        // Update the descriptor set to point to this buffer instead.
        vulkan_context.update_buffer_descriptor_set(
            &mesh.storage_buffer,
            mesh.descriptor_sets[0], // TODO: this isn't very dynamic..
            0,                       // also not dynamic
            vk::DescriptorType::STORAGE_BUFFER,
        );
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
    let mut query = Entity::query().filter(component::<Root>());
    let controller_entity = query.iter(world).next().unwrap().clone();

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
                            translations.push(vec3(t[0], t[1], t[2]));
                        }
                    }
                    Some(ReadOutputs::Rotations(rotation_data)) => {
                        for r in rotation_data.into_f32() {
                            rotations.push(Quaternion::new(r[3], r[0], r[1], r[2]));
                            // gltf gives us a quaternion in [x, y, z, w] but we need [w, x, y, z]
                        }
                    }
                    Some(ReadOutputs::Scales(scale_data)) => {
                        for s in scale_data {
                            scales.push(vec3(s[0], s[1], s[2]));
                        }
                    }
                    _ => {}
                }
            }

            let target_entity = node_entity_map.get(&target).unwrap();
            let mut target_entry = world.entry(*target_entity).unwrap();

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
            if let Ok(animation_target) = target_entry.get_component_mut::<AnimationTarget>() {
                animation_target.animations.push(animation);
            } else {
                target_entry.add_component(AnimationTarget {
                    controller: controller_entity.clone(),
                    animations: vec![animation],
                });
            }

            // Add an animation controller to our parent, if needed.
            let mut contoller_entry = world.entry(controller_entity).unwrap();
            if contoller_entry
                .get_component::<AnimationController>()
                .is_err()
            {
                contoller_entry.add_component(AnimationController::default())
            }
        }
    }
}

pub fn add_model_to_world(
    name: &str,
    models: &HashMap<String, World>,
    world: &mut World,
    parent: Option<Entity>,
) -> Option<Entity> {
    let mut merger = Duplicate::default();
    merger.register_clone::<Transform>();
    merger.register_clone::<TransformMatrix>();
    merger.register_clone::<Mesh>();
    merger.register_clone::<Skin>();
    merger.register_clone::<Joint>();
    merger.register_clone::<Parent>();
    merger.register_clone::<Entity>();
    merger.register_clone::<Root>();
    merger.register_clone::<Info>();
    merger.register_clone::<AnimationController>();
    merger.register_clone::<AnimationTarget>();

    let source = models.get(name)?;
    let mut query = Entity::query().filter(component::<Root>());
    let root_entity = query.iter(source).next().unwrap();

    let entity_map = world.clone_from(source, &any(), &mut merger);

    // If any entities had Parents, then let's fix up their relationships
    let mut query = <&mut Parent>::query();
    query.for_each_mut(world, |p| {
        // We have to make this conditional, as there may be entities from other models in the world.
        if let Some(new_parent) = entity_map.get(&p.0) {
            p.0 = *new_parent;
        }
    });

    // If any entities had Joints, then let's fix up their relationships
    let mut query = <&mut Joint>::query();
    query.for_each_mut(world, |j| {
        // We have to make this conditional, as there may be entities from other models in the world.
        if let Some(new_parent) = entity_map.get(&j.skeleton_root) {
            j.skeleton_root = *new_parent;
        }
    });

    // If any entities had AnimationTargets, then let's fix up their relationships
    let mut query = <&mut AnimationTarget>::query();
    query.for_each_mut(world, |a| {
        // We have to make this conditional, as there may be entities from other models in the world.
        if let Some(new_parent) = entity_map.get(&a.controller) {
            a.controller = *new_parent;
        }
    });

    let new_parent = entity_map.get(&root_entity).cloned().unwrap();

    if let Some(parent) = parent {
        let mut entity = world.entry(new_parent).unwrap();
        entity.add_component(Parent(parent));
    }

    Some(new_parent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        add_model_to_world,
        components::{Root, Transform},
        resources::{render_context::create_descriptor_set_layouts, VulkanContext},
    };
    use cgmath::assert_relative_eq;
    use legion::{EntityStore, IntoQuery};
    #[test]
    pub fn test_asteroid() {
        let vulkan_context = VulkanContext::testing().unwrap();
        let set_layouts = create_descriptor_set_layouts(&vulkan_context).unwrap();

        let gltf = include_bytes!("../../hotham-asteroid/assets/asteroid.gltf");
        let data = include_bytes!("../../hotham-asteroid/assets/asteroid_data.bin");
        let data: Vec<(&[u8], &[u8])> = vec![(gltf, data)];
        let models = load_models_from_gltf(data, &vulkan_context, set_layouts.mesh_layout).unwrap();

        let asteroid = models.get("Asteroid").unwrap();
        let mut query = <(&Mesh, &Transform, &TransformMatrix, &Info)>::query();
        let meshes = query.iter(asteroid).collect::<Vec<_>>();
        assert_eq!(meshes.len(), 1);

        let mut world = World::default();
        let asteroid = add_model_to_world("Asteroid", &models, &mut world, None);
        assert!(asteroid.is_some());

        let mut query = <(&Info, &Transform, &Mesh, &TransformMatrix, &Root)>::query();
        let asteroid = asteroid.unwrap();
        let (info, ..) = query.get(&mut world, asteroid).unwrap();
        assert_eq!(&info.name, "Asteroid");
        assert_eq!(info.node_id, 0);
    }

    #[test]
    pub fn test_hand() {
        let vulkan_context = VulkanContext::testing().unwrap();
        let set_layouts = create_descriptor_set_layouts(&vulkan_context).unwrap();

        let gltf = include_bytes!("../../hotham-asteroid/assets/left_hand.gltf");
        let data = include_bytes!("../../hotham-asteroid/assets/left_hand.bin");
        let data: Vec<(&[u8], &[u8])> = vec![(gltf, data)];
        let models = load_models_from_gltf(data, &vulkan_context, set_layouts.mesh_layout).unwrap();

        let mut world = World::default();
        let _hand = add_model_to_world("Left Hand", &models, &mut world, None);

        // Make sure there is only one root
        let mut query = <(&Root, &Info, &Transform)>::query();
        let roots = query.iter(&world).collect::<Vec<_>>();
        assert_eq!(roots.len(), 1);
        let root = roots[0];
        assert_eq!(&root.1.name, "Left Hand");

        // Make sure its transform is correct
        assert_relative_eq!(root.2.translation, vec3(0.0, 1.4, 0.0));
        assert_relative_eq!(root.2.rotation, Quaternion::new(0.707, 0.0, 0.0, 0.707));

        // Make sure we imported the mesh
        let mut query = <(&Mesh, &Transform, &TransformMatrix)>::query();
        let meshes = query.iter(&world).collect::<Vec<_>>();
        assert_eq!(meshes.len(), 1);

        // Make sure we imported the AnimationController
        let mut query = <&AnimationController>::query();
        let animation_controllers = query.iter(&world).collect::<Vec<_>>();
        assert_eq!(animation_controllers.len(), 1);

        // Make sure we got all the nodes
        let mut query = <&Transform>::query();
        let transforms = query.iter(&world).collect::<Vec<_>>();
        assert_eq!(transforms.len(), 28);

        // Make sure we got all the Parent -> Child relationships
        let mut query = <(&Transform, &Parent)>::query();
        let transforms_with_parents = query.iter(&world).collect::<Vec<_>>();
        assert_eq!(transforms_with_parents.len(), 27);

        // Make sure we got all the joints
        let mut query = <&Joint>::query();
        let joints = query.iter(&world).collect::<Vec<_>>();
        assert_eq!(joints.len(), 25);

        // Make sure we got all the AnimationTargets
        let mut query = <&AnimationTarget>::query();
        let animation_target = query.iter(&world).collect::<Vec<_>>();
        assert_eq!(animation_target.len(), 17);

        // Make sure the parent -> child relationships are correct
        let mut query = <&Parent>::query();
        unsafe {
            query.for_each_unchecked(&world, |parent| {
                let _ = world.entry_ref(parent.0).unwrap();
            });
        }

        // Make sure the joint -> skeleton_root relationships are correct
        let mut query = <&Joint>::query();
        unsafe {
            query.for_each_unchecked(&world, |joint| {
                let _ = world.entry_ref(joint.skeleton_root).unwrap();
            });
        }

        // Make sure the animation_target -> controller relationships are correct
        let mut query = <&AnimationTarget>::query();
        unsafe {
            query.for_each_unchecked(&world, |animation_target| {
                let _ = world.entry_ref(animation_target.controller).unwrap();
            });
        }
    }
}
