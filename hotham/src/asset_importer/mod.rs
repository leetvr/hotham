use crate::{
    components::{
        animation_controller::AnimationController, Info, Mesh, Parent, Root, Skin, Transform,
        TransformMatrix, Visible,
    },
    rendering::{material::Material, texture::Texture},
    resources::{RenderContext, VulkanContext},
};
use anyhow::Result;

use gltf::Document;
use hecs::{Entity, World};
use std::collections::HashMap;

/// Convenience type for models
pub type Models = HashMap<String, World>;

/// Convenience struct to hold all the necessary bits and pieces during the import of a single glTF file
pub(crate) struct ImportContext<'a> {
    pub vulkan_context: &'a VulkanContext,
    pub render_context: &'a mut RenderContext,
    pub models: Models,
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
    ) -> Self {
        let (document, mut buffers, images) = gltf::import_slice(glb_buffer).unwrap();

        let material_buffer_offset = render_context.resources.materials_buffer.len as _;
        Self {
            vulkan_context,
            render_context,
            models: Default::default(),
            node_entity_map: Default::default(),
            mesh_map: Default::default(),
            document,
            buffer: buffers.pop().unwrap(),
            images,
            material_buffer_offset,
        }
    }
}

/// Load glTF models from an array of GLB files.
pub fn load_models_from_glb(
    glb_buffers: &[&[u8]],
    vulkan_context: &VulkanContext,
    render_context: &mut RenderContext,
) -> Result<Models> {
    // Global models map, shared between imports.
    let mut models = HashMap::new();

    for glb_buffer in glb_buffers {
        let mut import_context = ImportContext::new(vulkan_context, render_context, glb_buffer);
        load_models_from_gltf_data(&mut import_context).unwrap();

        // Take all the models we imported and add them to the global map
        for (k, v) in import_context.models.drain() {
            models.insert(k, v);
        }
    }

    Ok(models)
}

/// Load glTF models from a glTF document
fn load_models_from_gltf_data(import_context: &mut ImportContext) -> Result<()> {
    // A bit lazy, but whatever.
    let document = import_context.document.clone();

    for mesh in document.meshes() {
        Mesh::load(mesh, import_context);
    }

    for material in document.materials() {
        Material::load(material, import_context);
    }

    for texture in document.textures() {
        Texture::load(texture, import_context);
    }

    // We need *some* entity to stash the AnimationController onto.
    // For now, just use the first root entity.
    let mut animation_controller_entity = None;
    for node in document.scenes().next().unwrap().nodes() {
        let mut world = World::default();

        let root = load_node(&node, import_context, &mut world, true);

        // Hacky
        if animation_controller_entity.is_none() {
            animation_controller_entity = Some(root);
        }

        add_parents(&node, &mut world, &mut import_context.node_entity_map);

        import_context
            .models
            .insert(node.name().expect("Node has no name!").to_string(), world);
    }

    // Finally, import any skins or animations.
    // Note that this has to be done after every single node has been imported, as skins and animations can reference any other node.

    // Skins are attached to nodes, so we need to go back through the node tree.
    for node in document.scenes().next().unwrap().nodes() {
        load_skins(node, import_context);
    }

    // TODO: This is *clearly* incorrect, and always was. Needs to be fixed if we want to support more than one animation per file.
    let animation_controller = AnimationController::load(document.animations(), import_context);
    let animation_controller_entity = animation_controller_entity.unwrap();
    // Find the world the entity belongs to.
    let world = import_context
        .models
        .values_mut()
        .find(|w| w.contains(animation_controller_entity))
        .unwrap();
    world
        .insert_one(animation_controller_entity, animation_controller)
        .unwrap();

    Ok(())
}

fn load_skins(node: gltf::Node, import_context: &mut ImportContext) {
    if let Some(skin) = node.skin() {
        // Load the skin
        let skin = Skin::load(skin, import_context);

        // Get the entity this node is mapped to
        let node_entity = import_context
            .node_entity_map
            .get(&node.index())
            .unwrap()
            .clone();

        // This part is tricky - we don't know which world this entity is in, so we need to look through them all
        let world = import_context
            .models
            .values_mut()
            .find(|w| w.contains(node_entity))
            .unwrap();

        world.insert_one(node_entity, skin).unwrap();
    }

    for node in node.children() {
        load_skins(node, import_context);
    }
}

#[cfg_attr(feature = "cargo-clippy", allow(clippy::too_many_arguments))]
fn load_node(
    node_data: &gltf::Node,
    import_context: &mut ImportContext,
    world: &mut World,
    is_root: bool,
) -> Entity {
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
        .insert(node_data.index(), this_entity.clone());

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

    this_entity
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

    println!("Adding {} to world", name);

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

        // If the source entity had a skin, insert it into the new world.
        // Right now inserting a model with a skin into the world more than once is not supported. This is because
        // we would have to allocate a new skin_id, which would require some mucking about with our buffers.
        if let Ok(skin) = source_world.get_mut::<Skin>(*source_entity) {
            let mut new_skin = skin.clone();

            // Go through each of the joints and map them to their new entities.
            new_skin
                .joints
                .iter_mut()
                .for_each(|e| *e = entity_map.get(&e).cloned().unwrap());

            destination_world
                .insert_one(*destination_entity, new_skin)
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
            // TODO: Is this neccessary?
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
            let mut new_animation_controller = animation_controller.clone();

            // Go through each of the joints and map them to their new entities.
            new_animation_controller
                .targets
                .iter_mut()
                .for_each(|t| t.target = entity_map.get(&t.target).cloned().unwrap());

            destination_world
                .insert_one(*destination_entity, new_animation_controller)
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

    destination_world.flush();

    Some(new_root_entity)
}

#[cfg(target_os = "windows")]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Root, Transform};
    use approx::assert_relative_eq;
    use nalgebra::{vector, Quaternion, UnitQuaternion};

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
                "Damaged Helmet",
                0,
                46356,
                vector![0., 1.4, 0.],
                Quaternion::new(0.707, 0.707, 0., 0.),
            ),
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

            // Ensure we imported the material correctly
            if *name == "Damaged Helmet" {
                unsafe {
                    let material = &render_context.resources.materials_buffer.as_slice()
                        [primitive.material_id as usize];
                    assert_eq!(material.base_color_texture_set, 0);
                    assert_eq!(material.physical_descrtiptor_texture_id, 1);
                    assert_eq!(material.normal_texture_set, 2);
                    assert_eq!(material.occlusion_texture_set, 3);
                }
            }

            // Ensure the transform was populated correctly
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

        // Make sure we got the skin
        {
            let mut query = world.query::<&Skin>();
            let (_, skin) = query.iter().next().unwrap();
            for joint in skin.joints.iter() {
                assert!(world.contains(*joint));
            }
        }

        // Make sure we imported the AnimationController
        {
            let mut query = world.query::<&AnimationController>();
            assert_eq!(query.iter().len(), 1);
            let (_, animation_controller) = query.iter().next().unwrap();
            for target in animation_controller.targets.iter() {
                assert!(world.contains(target.target));
            }
        }
    }
}
