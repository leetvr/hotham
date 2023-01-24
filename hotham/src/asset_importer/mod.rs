/// Representation of a glTF Scene
pub mod scene;

use crate::{
    components::{
        animation_controller::AnimationController, Collider, GlobalTransform, Info, LocalTransform,
        Mesh, Parent, Root, Skin, Visible,
    },
    contexts::{
        physics_context::{self},
        RenderContext, VulkanContext,
    },
    rendering::{light::Light, material::Material},
};
use anyhow::Result;

use glam::{Affine3A, Mat4};
use gltf::Document;
use hecs::{Entity, World};
use itertools::Itertools;
use rapier3d::prelude::ActiveCollisionTypes;
use std::{borrow::Cow, collections::HashMap, convert::TryInto};

use self::scene::Scene;

static COLLIDER_TAG: &str = ".HOTHAM_COLLIDER";
static WALL_COLLIDER_TAG: &str = ".HOTHAM_COLLIDER_WALL";
static SENSOR_COLLIDER_TAG: &str = ".HOTHAM_COLLIDER_SENSOR";

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
    pub buffer: Cow<'a, [u8]>,
    pub material_buffer_offset: u32,
}

impl<'a> ImportContext<'a> {
    fn new(
        vulkan_context: &'a VulkanContext,
        render_context: &'a mut RenderContext,
        glb_buffer: &'a [u8],
    ) -> Self {
        let glb = gltf::Glb::from_slice(glb_buffer).unwrap();
        let json = gltf::json::Root::from_slice(&glb.json).unwrap();
        let document = gltf::Document::from_json_without_validation(json);
        let buffer = glb.bin.unwrap();

        let material_buffer_offset = render_context.resources.materials_buffer.len as _;
        Self {
            vulkan_context,
            render_context,
            models: Default::default(),
            node_entity_map: Default::default(),
            mesh_map: Default::default(),
            document,
            buffer,
            material_buffer_offset,
        }
    }
}

/// Load glTF scene from a GLB file
pub fn load_scene_from_glb(
    glb_buffer: &[u8],
    vulkan_context: &VulkanContext,
    render_context: &mut RenderContext,
) -> Result<Scene> {
    // Global models map, shared between imports.
    let mut models = HashMap::new();

    let mut import_context = ImportContext::new(vulkan_context, render_context, glb_buffer);
    load_models_from_gltf_data(&mut import_context).unwrap();

    // Take all the models we imported and add them to the global map
    for (k, v) in import_context.models.drain() {
        models.insert(k, v);
    }

    let lights = get_lights_from_gltf_data(&import_context.document)?;

    Ok(Scene { models, lights })
}

// TODO: At the moment we only support lights in the top level scene object. glTF lets us do fancier things like
//       have lights be part of the node hierarchy, which we should definitely support, but we're not there yet.
fn get_lights_from_gltf_data(document: &Document) -> Result<Vec<Light>> {
    let mut lights = Vec::new();
    for node in document
        .default_scene()
        .ok_or_else(|| anyhow::format_err!("glTF file does not have a default scene!"))?
        .nodes()
    {
        if let Some(light) = node.light() {
            lights.push(Light::from_gltf(&light, &node));
        }
    }

    Ok(lights)
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

    // Identify meshes that will be used for collider geometry.
    let collider_mesh_ids = get_collider_mesh_ids(document.nodes());

    for mesh in document.meshes() {
        // Don't load meshes that are going to be used as collider geometry
        if collider_mesh_ids.contains(&mesh.index()) {
            continue;
        }

        Mesh::load(mesh, import_context);
    }

    for material in document.materials() {
        Material::load(material, import_context);
    }

    // We need *some* entity to stash the AnimationController onto.
    // For now, just use the first root entity.
    let mut animation_controller_entity = None;

    // NOTE: We don't currently support multiple scenes, so we just take the first one.
    let scene = document.scenes().next().unwrap();

    // Iterate through each of the root nodes in the scene and load it in.
    for node in scene.nodes() {
        // Don't add wall collider geometry as nodes.
        if node.name().unwrap_or_default().ends_with(WALL_COLLIDER_TAG) {
            continue;
        }

        let mut world = World::default();

        let root = load_node(&node, import_context, &mut world, true);

        // Hacky
        if animation_controller_entity.is_none() {
            animation_controller_entity = Some(root);
        }

        build_node_hierarchy(&node, &mut world, &mut import_context.node_entity_map);

        import_context
            .models
            .insert(node.name().expect("Node has no name!").to_string(), world);
    }

    // Finally, import any skins or animations.
    //
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

fn get_collider_mesh_ids(nodes: gltf::iter::Nodes) -> Vec<usize> {
    let mut mesh_ids = Vec::new();
    for node in nodes {
        if !node.name().unwrap_or_default().contains(COLLIDER_TAG) {
            continue;
        }

        if let Some(mesh) = node.mesh() {
            mesh_ids.push(mesh.index())
        }
    }

    mesh_ids
}

fn load_skins(node: gltf::Node, import_context: &mut ImportContext) {
    if let Some(skin) = node.skin() {
        // Load the skin
        let skin = Skin::load(skin, import_context);

        // Get the entity this node is mapped to
        let node_entity = *import_context.node_entity_map.get(&node.index()).unwrap();

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
    node: &gltf::Node,
    import_context: &mut ImportContext,
    world: &mut World,
    is_root: bool,
) -> Entity {
    // First, get the transform of the node.
    let local_transform = LocalTransform::load(node.transform());

    let matrix = Mat4::from_cols_array_2d(&node.transform().matrix());
    let global_transform = GlobalTransform(Affine3A::from_mat4(matrix));

    // Next, collect some information about the node and store it in an [`Info`] component
    let info = Info {
        name: node
            .name()
            .map(|s| s.to_string())
            .unwrap_or(format!("Node {}", node.index())),
        node_id: node.index(),
    };

    // Now spawn an entity to represent this node and store it in our entity map.
    let this_entity = world.spawn((local_transform, global_transform, info));
    import_context
        .node_entity_map
        .insert(node.index(), this_entity);

    // If the node had a mesh, add the mesh as a component and give it a `Visible` component
    if let Some(mesh) = node
        .mesh()
        .and_then(|m| import_context.mesh_map.get(&m.index()))
    {
        world
            .insert(this_entity, (mesh.clone(), Visible {}))
            .unwrap();
    }

    // If this node is at the root, mark it with a `Root` component.
    if is_root {
        world.insert_one(this_entity, Root {}).unwrap();
    }

    // If this node has corresponding collider geometry, add it in.
    if let Some(collider) = get_collider_for_node(node, import_context) {
        world.insert_one(this_entity, collider).unwrap();
    }

    // Now walk through each of this node's children and load them in.
    for child in node.children() {
        load_node(&child, import_context, world, false);
    }

    this_entity
}

/// Searches through the glTF document to find a mesh that can be used by Hotham to represent a collider, then creates one.
///
/// There are two kinds of colliders we're looking for:
///
/// - Walls, which are stored separate node with the same root name as some entity, eg. `cube` and `cube.HOTHAM_COLLIDER_WALL`
/// - Sensors, which are their own separate nodes, eg. `phantom.HOTHAM_COLLIDER_SENSOR`
fn get_collider_for_node(
    node: &gltf::Node,
    import_context: &mut ImportContext,
) -> Option<Collider> {
    // First, get the name of the node, if it has one.
    let node_name = node.name()?;

    // Next, check to see if this is either a node that should be treated as a sensor
    // OR a node that has another node representing a wall collider somewhere in the document.
    let (collider_node_name, mesh) = if node_name.ends_with(SENSOR_COLLIDER_TAG) {
        (node_name, node.mesh()?)
    } else {
        find_wall_collider_for_node(node_name, import_context)?
    };

    // Build a collider using the mesh.
    println!(
        "[HOTHAM_ASSET_IMPORTER] Getting shape for {}",
        collider_node_name
    );
    let shape = get_shape_from_mesh(mesh, import_context);

    // If this is a wall collider, ensure it's not a sensor.
    let collider = if collider_node_name.ends_with(WALL_COLLIDER_TAG) {
        println!(
            "[HOTHAM_ASSET_IMPORTER] Created wall collider for model {}",
            collider_node_name
        );
        Collider {
            sensor: false,
            collision_groups: physics_context::WALL_COLLISION_GROUP,
            collision_filter: u32::MAX,
            shape,
            ..Default::default()
        }
    } else {
        println!(
            "[HOTHAM_ASSET_IMPORTER] Created sensor collider for model {}",
            collider_node_name
        );
        Collider {
            sensor: true,
            collision_groups: physics_context::SENSOR_COLLISION_GROUP,
            collision_filter: u32::MAX,
            active_collision_types: ActiveCollisionTypes::all(),
            shape,
            ..Default::default()
        }
    };

    Some(collider)
}

fn find_wall_collider_for_node<'a>(
    name: &'a str,
    import_context: &'a ImportContext,
) -> Option<(&'a str, gltf::Mesh<'a>)> {
    // Create a pattern to search for the collider's name, suffixed with the wall collider tag.
    let wall_pattern = format!("{}{}", name, WALL_COLLIDER_TAG);

    // Iterate through each node to try and find the matching node, then fetch its mesh.
    import_context.document.nodes().find_map(|n| {
        if n.name()? != wall_pattern {
            return None;
        }

        n.mesh().map(|mesh| (n.name().unwrap(), mesh))
    })
}

/// Use Rapier's convex_decomposition to create a shape from the mesh geometry.
fn get_shape_from_mesh(
    mesh: gltf::Mesh,
    import_context: &ImportContext,
) -> rapier3d::geometry::SharedShape {
    let mut positions = Vec::new();
    let mut indices: Vec<[u32; 3]> = Default::default();

    for primitive in mesh.primitives() {
        let reader = primitive.reader(|_| Some(&import_context.buffer));
        if let Some(iter) = reader.read_positions() {
            for p in iter {
                positions.push(p.into());
            }
        } else {
            panic!("[HOTHAM_ASSET_IMPORTER] - Unable to create collider, mesh has no positions!");
        }

        if let Some(iter) = reader.read_indices() {
            for chunk in &iter.into_u32().chunks(3) {
                indices.push(chunk.collect::<Vec<_>>().try_into().expect(
                    "[HOTHAM_ASSET_IMPORTER] - Unable to create collider, invalid geometry!",
                ));
            }
        } else {
            panic!("[HOTHAM_ASSET_IMPORTER] - Unable to create collider, mesh has no positions!");
        }
    }

    println!(
        "[HOTHAM_ASSET_IMPORTER] Attempting to create convex mesh from {:?} positions",
        positions.len()
    );

    rapier3d::geometry::SharedShape::convex_mesh(positions.clone(), &indices).unwrap_or_else(|| {
        println!(
            "[HOTHAM_ASSET_IMPORTER] ERROR! Unable to create convex mesh, attempting decomposition"
        );
        rapier3d::geometry::SharedShape::convex_decomposition(&positions, &indices)
    })
}

/// Recursively walk through this node's hierarchy and connect child nodes to their parents by adding a [`Parent`] component.
///
/// **NOTE**: We only support very minimal parent -> child inheritance. At present only visibilty and transforms
///       are inherited.
fn build_node_hierarchy(
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
        build_node_hierarchy(&child_node, world, node_entity_map);
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
        let source_entity = source_world.entity(*source_entity).unwrap();

        if let Some(local_transform) = source_entity.get::<&LocalTransform>() {
            destination_world
                .insert_one(*destination_entity, *local_transform)
                .unwrap();
        }

        if let Some(global_transform) = source_entity.get::<&GlobalTransform>() {
            destination_world
                .insert_one(*destination_entity, *global_transform)
                .unwrap();
        }

        // Create a new mesh for this entity in the destination world.
        if let Some(mesh) = source_entity.get::<&Mesh>() {
            destination_world
                .insert_one(*destination_entity, (*mesh).clone())
                .unwrap();
        }

        // If the source entity had a skin, insert it into the new world.
        // Right now inserting a model with a skin into the world more than once is not supported. This is because
        // we would have to allocate a new skin_id, which would require some mucking about with our buffers.
        if let Some(skin) = source_entity.get::<&Skin>() {
            let mut new_skin = (*skin).clone();

            // Go through each of the joints and map them to their new entities.
            new_skin
                .joints
                .iter_mut()
                .for_each(|e| *e = entity_map.get(e).cloned().unwrap());

            destination_world
                .insert_one(*destination_entity, new_skin)
                .unwrap();
        }

        // If the source entity had a parent, set it to the corresponding entity in the destination world.
        if let Some(parent) = source_entity.get::<&Parent>() {
            let new_parent = entity_map.get(&parent.0).unwrap();
            destination_world
                .insert_one(*destination_entity, Parent(*new_parent))
                .unwrap();
        }

        if let Some(root) = source_entity.get::<&Root>() {
            destination_world
                .insert_one(*destination_entity, *root)
                .unwrap();

            // Set a parent for the root entity if one was specified.
            // TODO: Is this necessary?
            if let Some(parent) = parent {
                destination_world
                    .insert_one(*destination_entity, Parent(parent))
                    .unwrap();
            }
        }

        if let Some(info) = source_entity.get::<&Info>() {
            destination_world
                .insert_one(*destination_entity, (*info).clone())
                .unwrap();
        }

        if let Some(animation_controller) = source_entity.get::<&AnimationController>() {
            let mut new_animation_controller = (*animation_controller).clone();

            // Go through each of the joints and map them to their new entities.
            new_animation_controller
                .targets
                .iter_mut()
                .for_each(|t| t.target = entity_map.get(&t.target).cloned().unwrap());

            destination_world
                .insert_one(*destination_entity, new_animation_controller)
                .unwrap();
        }

        if let Some(visible) = source_entity.get::<&Visible>() {
            destination_world
                .insert_one(*destination_entity, *visible)
                .unwrap();
        }

        // If the entity had a collider attached, clone it and insert it into the new world. Its underlying will be handled by `PhysicsContext`.
        if let Some(collider) = source_entity.get::<&Collider>() {
            destination_world
                .insert_one(*destination_entity, (*collider).clone())
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

// These tests are disabled for other platforms
// https://github.com/leetvr/hotham/issues/240
#[cfg(target_os = "windows")]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{LocalTransform, Root};
    use approx::assert_relative_eq;
    use glam::Quat;

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
                [0., 0., 0.].into(),
                Quat::from_xyzw(0.70710677, 0., 0., 0.70710677),
            ),
            (
                "Asteroid",
                0,
                1800,
                [0., 0., 0.].into(),
                Quat::from_xyzw(0., 0., 0., 1.),
            ),
            (
                "Refinery",
                1,
                23928,
                [-0.06670809, 2.1408155, -0.46151406].into(),
                Quat::from_xyzw(
                    -0.09325116872787476,
                    0.6883626580238342,
                    0.006518156733363867,
                    0.719318151473999,
                ),
            ),
        ];

        for (name, id, indices_count, translation, rotation) in &test_data {
            let _ = models
                .get(*name)
                .expect(&format!("Unable to find model with name {}", name));

            let mut world = World::default();
            let model = add_model_to_world(*name, &models, &mut world, None);
            assert!(model.is_some(), "Model {} could not be added", name);

            let model = model.unwrap();
            let (info, local_transform, mesh, ..) = world
                .query_one_mut::<(&Info, &LocalTransform, &Mesh, &GlobalTransform, &Root)>(model)
                .unwrap();
            let mesh = render_context.resources.mesh_data.get(mesh.handle).unwrap();
            let primitive = &mesh.primitives[0];
            assert_eq!(primitive.indices_count, *indices_count as u32);

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
                    assert_eq!(material.packed_flags_and_base_texture_id >> 16, 1);
                }
            }

            // Ensure the transform was populated correctly
            assert_relative_eq!(local_transform.translation, *translation,);
            assert_relative_eq!(local_transform.rotation, *rotation,);
            assert_eq!(&info.name, *name);
            assert_eq!(&info.node_id, id, "Node {} has wrong ID", name);
        }
    }

    #[test]
    fn test_load_model_with_no_material() {
        let (mut render_context, vulkan_context) = RenderContext::testing();
        let data: Vec<&[u8]> = vec![include_bytes!("../../../test_assets/box_no_material.glb")];
        let _models = load_models_from_glb(&data, &vulkan_context, &mut render_context).unwrap();
    }

    #[test]
    pub fn test_hand() {
        let (mut render_context, vulkan_context) = RenderContext::testing();

        let data: Vec<&[u8]> = vec![include_bytes!("../../../test_assets/left_hand.glb")];
        let models = load_models_from_glb(&data, &vulkan_context, &mut render_context).unwrap();

        let mut world = World::default();
        let _hand = add_model_to_world("Left Hand", &models, &mut world, None);

        // Make sure there is only one root
        let mut roots = world
            .query_mut::<(&Root, &Info, &LocalTransform)>()
            .into_iter();
        assert_eq!(roots.len(), 1);
        let root = roots.next().unwrap().1;
        assert_eq!(&root.1.name, "Left Hand");

        // Make sure its transform is correct
        assert_relative_eq!(root.2.translation, [0.0, 0.0, 0.0].into());

        // Make sure we imported the mesh
        let meshes = world
            .query_mut::<(&Mesh, &LocalTransform, &GlobalTransform)>()
            .into_iter();
        assert_eq!(meshes.len(), 1);

        // Make sure we got all the nodes
        let transforms = world.query_mut::<&LocalTransform>().into_iter();
        assert_eq!(transforms.len(), 27);

        // Make sure we got all the Parent -> Child relationships
        {
            let mut transforms_with_parents = world.query::<(&LocalTransform, &Parent)>();
            assert_eq!(transforms_with_parents.iter().len(), 26);
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

    #[test]
    fn test_load_model_with_colliders() {
        let (mut render_context, vulkan_context) = RenderContext::testing();
        let mut world = World::default();

        let data: Vec<&[u8]> = vec![include_bytes!(
            "../../../test_assets/box_with_colliders.glb"
        )];
        let models = load_models_from_glb(&data, &vulkan_context, &mut render_context).unwrap();
        for name in models.keys() {
            add_model_to_world(name, &models, &mut world, None);
        }

        // There are two wolv-- colliders.
        let query = world.query_mut::<&Collider>();
        assert_eq!(query.into_iter().len(), 2);

        // Make sure we got the wall collider
        let query = world.query_mut::<(&Collider, &Mesh)>();
        assert_eq!(query.into_iter().len(), 1);

        // ..and make sure we got the sensor collider
        let query = world.query_mut::<hecs::Without<&mut Collider, &Mesh>>();
        assert_eq!(query.into_iter().len(), 1);
    }
}
