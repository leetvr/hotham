use crate::{
    buffer::Buffer,
    components::{
        animation_controller::AnimationController, mesh::MeshUBO, AnimationTarget, Info, Joint,
        Mesh, Parent, Root, Skin, Transform, TransformMatrix, Visible,
    },
    resources::{render_context::DescriptorSetLayouts, VulkanContext},
};
use anyhow::Result;
use ash::vk;
use gltf::animation::util::ReadOutputs;
use itertools::{izip, Itertools};
use legion::{any, component, world::Duplicate, Entity, IntoQuery, World};
use nalgebra::{vector, Matrix4, Quaternion, UnitQuaternion};
use std::collections::HashMap;

pub fn load_models_from_glb(
    glb_bufs: &Vec<&[u8]>,
    vulkan_context: &VulkanContext,
    descriptor_set_layouts: &DescriptorSetLayouts,
) -> Result<HashMap<String, World>> {
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

pub fn load_models_from_gltf_data(
    document: &gltf::Document,
    buffer: &[u8],
    images: &Vec<gltf::image::Data>,
    vulkan_context: &VulkanContext,
    descriptor_set_layouts: &DescriptorSetLayouts,
    models: &mut HashMap<String, World>,
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
    let this_entity = world.push((transform, transform_matrix, info));
    node_entity_map.insert(node_data.index(), this_entity);
    let mut e = world.entry(this_entity).unwrap();

    if let Some(mesh) = node_data.mesh() {
        let mesh = Mesh::load(
            &mesh,
            gltf_buffer,
            vulkan_context,
            descriptor_set_layouts,
            images,
        )?;

        e.add_component(mesh);
        e.add_component(Visible {})
    }

    if is_root {
        e.add_component(Root {});
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
    // TODO: Extract this to components::Skin
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

        // Tell the vertex shader how many joints we have
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

            let target_entity = node_entity_map.get(&target).unwrap();
            let mut target_entry = if let Some(target_entry) = world.entry(*target_entity) {
                target_entry
            } else {
                println!("[HOTHAM_GLTF] - Error importing animation {:?}. No target, probably due to malformed file. Ignoring", animation.name());
                return;
            };

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
    vulkan_context: &VulkanContext,
    descriptor_set_layouts: &DescriptorSetLayouts,
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
    merger.register_clone::<Visible>();

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

    let new_entity = entity_map.get(&root_entity).cloned().unwrap();

    // Optionally set a new parent for this entity, if it was passed in as a parameter.
    if let Some(parent) = parent {
        let mut entity = world.entry(new_entity).unwrap();
        entity.add_component(Parent(parent));
    }

    // We'll need to create a new UBO for the mesh:
    let mut query = <(&Info, &mut Mesh)>::query();
    let (info, mesh) = query.get_mut(world, new_entity).unwrap();

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

    Some(new_entity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        components::{Root, Transform},
        resources::{render_context::create_descriptor_set_layouts, VulkanContext},
    };
    use approx::assert_relative_eq;
    use legion::{EntityStore, IntoQuery};

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
            let mut query = <(&Mesh, &Transform, &TransformMatrix, &Info)>::query();
            let meshes = query.iter(model_world).collect::<Vec<_>>();
            assert_eq!(meshes.len(), 1);

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

            let mut query = <(&Info, &Transform, &Mesh, &TransformMatrix, &Root)>::query();
            let model = model.unwrap();
            let (info, transform, mesh, ..) = query.get(&mut world, model).unwrap();
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
            let initial_buffer = meshes[0].0.ubo_buffer.handle;
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
        let mut query = <(&Root, &Info, &Transform)>::query();
        let roots = query.iter(&world).collect::<Vec<_>>();
        assert_eq!(roots.len(), 1);
        let root = roots[0];
        assert_eq!(&root.1.name, "Left Hand");

        // Make sure its transform is correct
        assert_relative_eq!(root.2.translation, vector![0.0, 0.0, 0.0]);

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
