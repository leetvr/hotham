use crate::{
    components::{
        animation_controller::AnimationController, AnimationTarget, Info, Joint, Mesh, Parent,
        Root, Skin, Transform, TransformMatrix, Visible,
    },
    rendering::{material::Material, mesh_data::MeshData, resources::Resources, texture::Texture},
    resources::{RenderContext, VulkanContext},
};
use anyhow::Result;

use generational_arena::Index;
use gltf::{animation::util::ReadOutputs, Document};
use hecs::{Entity, World};
use id_arena::Id;
use itertools::{izip, Itertools};
use nalgebra::{vector, Matrix4, Quaternion, UnitQuaternion};
use std::collections::HashMap;

/// Convenience type for models
pub type Models = HashMap<String, World>;

/// Convenience struct to hold all the necessary bits and pieces during the import of a single glTF file
pub(crate) struct ImportContext<'a> {
    pub vulkan_context: &'a VulkanContext,
    pub render_context: &'a mut RenderContext,
    pub models: &'a mut Models,
    pub node_entity_map: HashMap<usize, Entity>,
    pub mesh_map: HashMap<usize, Mesh>,
    pub document: Document,
    pub buffer: gltf::buffer::Data,
    pub images: Vec<gltf::image::Data>,
    pub material_buffer_offset: u32,
}

impl<'a> ImportContext<'a> {
    fn new(
        vulkan_context: &'a VulkanContext,
        render_context: &'a mut RenderContext,
        glb_buffer: &'a [u8],
        models: &'a mut Models,
    ) -> Self {
        let (document, mut buffers, images) = gltf::import_slice(glb_buffer).unwrap();

        let material_buffer_offset = render_context.resources.materials_buffer.len as _;
        Self {
            vulkan_context,
            render_context,
            models,
            node_entity_map: Default::default(),
            mesh_map: Default::default(),
            document,
            buffer: buffers.pop().unwrap(),
            images,
            material_buffer_offset,
        }
    }
}

/// Load glTF models from a GLB file
pub fn load_models_from_glb(
    glb_buffers: &[&[u8]],
    vulkan_context: &VulkanContext,
    render_context: &mut RenderContext,
) -> Result<Models> {
    let mut models = HashMap::new();

    for glb_buffer in glb_buffers {
        let mut import_context =
            ImportContext::new(vulkan_context, render_context, glb_buffer, &mut models);
        load_models_from_gltf_data(&mut import_context).unwrap();
    }

    Ok(models)
}

/// Load glTF models from a glTF document
fn load_models_from_gltf_data(import_context: &mut ImportContext) -> Result<()> {
    // A bit lazy, but whatever.
    let document = import_context.document.clone();

    // Previously, we assumed nodes were the centre of the universe. That is untrue.
    // Instead, we'll import each resource type individually, updating references as we go.
    for mesh in document.meshes() {
        Mesh::load(mesh, import_context);
    }

    for material in document.materials() {
        Material::load(material, import_context);
    }

    for texture in document.textures() {
        Texture::load(texture, import_context);
    }

    for node_data in document.scenes().next().unwrap().nodes() {
        let mut world = World::default();

        load_node(&node_data, import_context, &mut world, true);
        add_parents(&node_data, &mut world, &mut import_context.node_entity_map);
        // add_skins_and_joints(
        //     &node_data,
        //     buffer,
        //     &mut world,
        //     vulkan_context,
        //     &mut node_entity_map,
        // );
        // add_animations(&animations, buffer, &mut world, &mut node_entity_map);

        import_context.models.insert(
            node_data.name().expect("Node has no name!").to_string(),
            world,
        );
    }

    Ok(())
}

#[cfg_attr(feature = "cargo-clippy", allow(clippy::too_many_arguments))]
fn load_node(
    node_data: &gltf::Node,
    import_context: &mut ImportContext,
    world: &mut World,
    is_root: bool,
) {
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
    import_context
        .node_entity_map
        .insert(node_data.index(), this_entity);

    if let Some(mesh) = node_data
        .mesh()
        .and_then(|m| import_context.mesh_map.get(&m.index()))
    {
        world
            .insert(this_entity, (mesh.clone(), Visible {}))
            .unwrap();
    }

    if is_root {
        world.insert_one(this_entity, Root {}).unwrap();
    }

    for child in node_data.children() {
        load_node(&child, import_context, world, false);
    }
}

fn add_parents(
    node_data: &gltf::Node,
    world: &mut World,
    node_entity_map: &mut HashMap<usize, Entity>,
) {
    let this_entity = node_entity_map.get(&node_data.index()).unwrap();
    let parent = Parent(*this_entity);
    for child_node in node_data.children() {
        let child_id = child_node.index();
        let child_entity = node_entity_map.get(&child_id).unwrap();
        world.insert_one(*child_entity, parent).unwrap();
        add_parents(&child_node, world, node_entity_map);
    }
}

// fn add_skins_and_joints(
//     node_data: &gltf::Node,
//     buffer: &[u8],
//     world: &mut World,
//     vulkan_context: &VulkanContext,
//     node_entity_map: &mut HashMap<usize, Entity>,
// ) {
//     // Do we need to add a Skin?
//     // TODO: Extract this to components::Skin
//     if let Some(node_skin_data) = node_data.skin() {
//         println!("[HOTHAM_GLTF] Adding a skin to {}", node_data.index());
//         let this_entity = *node_entity_map.get(&node_data.index()).unwrap();
//         let mut joint_matrices = Vec::new();
//         let reader = node_skin_data.reader(|_| Some(buffer));
//         let matrices = reader.read_inverse_bind_matrices().unwrap();
//         for m in matrices {
//             let m = Matrix4::from(m);
//             joint_matrices.push(m);
//         }
//         let mut joint_ids = Vec::new();

//         for (joint_node, inverse_bind_matrix) in node_skin_data.joints().zip(joint_matrices.iter())
//         {
//             let joint = Joint {
//                 skeleton_root: this_entity,
//                 inverse_bind_matrix: *inverse_bind_matrix,
//             };
//             joint_ids.push(joint_node.index());
//             let joint_entity = node_entity_map.get(&joint_node.index()).unwrap();
//             world.insert_one(*joint_entity, joint).unwrap();
//         }

//         // Add a Skin to the entity.
//         world.insert_one(this_entity, Skin { joint_ids }).unwrap();

//         // Tell the vertex shader how many joints we have
//         let mut mesh = world.get_mut::<Mesh>(this_entity).unwrap();
//         mesh.ubo_data.joint_count = joint_matrices.len() as f32;
//     }

//     for child in node_data.children() {
//         add_skins_and_joints(&child, buffer, world, vulkan_context, node_entity_map);
//     }
// }

fn add_animations(
    animations: &[gltf::Animation], // Clippy ptr_arg
    buffer: &[u8],
    world: &mut World,
    node_entity_map: &mut HashMap<usize, Entity>,
) {
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
                                controller: controller_entity,
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
                .insert_one(*destination_entity, *transform)
                .unwrap();
        }

        if let Ok(transform_matrix) = source_world.get_mut::<TransformMatrix>(*source_entity) {
            destination_world
                .insert_one(*destination_entity, *transform_matrix)
                .unwrap();
        }

        // Create a new mesh for this entity in the destination world.
        if let Ok(mesh) = source_world.get_mut::<Mesh>(*source_entity) {
            destination_world
                .insert_one(*destination_entity, mesh.clone())
                .unwrap();
        }

        if let Ok(skin) = source_world.get_mut::<Skin>(*source_entity) {
            destination_world
                .insert_one(*destination_entity, skin.clone())
                .unwrap();
        }

        // If the source entity had a joint, clone it and set the skeleton root to the corresponding entity in the destination world.
        if let Ok(joint) = source_world.get_mut::<Joint>(*source_entity) {
            let mut new_joint = *joint;
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
                .insert_one(*destination_entity, *root)
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
                .insert_one(*destination_entity, *visible)
                .unwrap();
        }
    }

    // Find the root entity of the source world.
    let (root_entity, _) = source_world.query::<&Root>().iter().next().unwrap();

    // Get the new root entity.
    let new_root_entity = entity_map.get(&root_entity).cloned().unwrap();

    Some(new_root_entity)
}

#[cfg(target_os = "windows")]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        components::{Root, Transform},
        resources::{vulkan_context, VulkanContext},
    };
    use approx::assert_relative_eq;

    #[test]
    pub fn test_load_models() {
        let (mut render_context, vulkan_context) = RenderContext::testing();

        let data: Vec<&[u8]> = vec![
            include_bytes!("../../../test_assets/damaged_helmet.glb"),
            include_bytes!("../../../test_assets/asteroid.glb"),
        ];
        let models = load_models_from_glb(&data, &vulkan_context, &mut render_context).unwrap();
        let test_data = vec![
            (
                "Asteroid",
                0,
                1800,
                vector![0., 0., 0.],
                Quaternion::new(1., 0., 0., 0.),
            ),
            (
                "Refinery",
                1,
                23928,
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
                46356,
                vector![0., 1.4, 0.],
                Quaternion::new(0.707, 0.707, 0., 0.),
            ),
        ];
        for (name, id, indicies_count, translation, rotation) in &test_data {
            let _ = models
                .get(*name)
                .expect(&format!("Unable to find model with name {}", name));

            let mut world = World::default();
            let model = add_model_to_world(*name, &models, &mut world, None);
            assert!(model.is_some(), "Model {} could not be added", name);

            let model = model.unwrap();
            let (info, transform, mesh, ..) = world
                .query_one_mut::<(&Info, &Transform, &Mesh, &TransformMatrix, &Root)>(model)
                .unwrap();
            let mesh = render_context.resources.mesh_data.get(mesh.handle).unwrap();
            let primitive = &mesh.primitives[0];
            assert_eq!(primitive.indices_count, *indicies_count as u32);

            // Ensure we populated the buffers correctly.
            unsafe {
                let vertex_buffer = render_context.resources.vertex_buffer.as_slice();
                let index_buffer = render_context.resources.index_buffer.as_slice();
                for n in 0..primitive.indices_count as _ {
                    let index = index_buffer[(primitive.index_buffer_offset + n) as usize] as usize;
                    let _vertex = &vertex_buffer[index];
                }
            }
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
        }
    }

    #[test]
    pub fn test_hand() {
        let (mut render_context, vulkan_context) = RenderContext::testing();

        let data: Vec<&[u8]> = vec![include_bytes!("../../../test_assets/left_hand.glb")];
        let models = load_models_from_glb(&data, &vulkan_context, &mut render_context).unwrap();

        let mut world = World::default();
        let _hand = add_model_to_world("Left Hand", &models, &mut world, None);

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
