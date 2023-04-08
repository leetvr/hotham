use enum_iterator::{all, Sequence};

use hotham::{
    asset_importer::add_model_to_world,
    components::{physics::SharedShape, Collider, LocalTransform, Stage},
    glam::{vec3a, Vec3A},
    hecs::World,
    Engine,
};

#[derive(Debug, PartialEq, Sequence)]
pub enum IkNodeID {
    LeftGrip,
    LeftAim,
    RightGrip,
    RightAim,
    Hmd,
    Root,
}

pub struct IkNode {
    node_id: IkNodeID,
}

pub fn add_ik_nodes(models: &std::collections::HashMap<String, World>, world: &mut World) {
    let collider = Collider::new(SharedShape::ball(0.1));
    for node_id in all::<IkNodeID>() {
        let entity = add_model_to_world("Axes", models, world, None).unwrap();
        world
            .insert(entity, (collider.clone(), IkNode { node_id }))
            .unwrap();
    }
    let stages = world
        .query::<&Stage>()
        .iter()
        .map(|(entity, _)| entity)
        .collect::<Vec<_>>();
    for parent in stages {
        for node_id in all::<IkNodeID>() {
            let entity = add_model_to_world("Axes", models, world, Some(parent)).unwrap();
            world
                .insert(entity, (collider.clone(), IkNode { node_id }))
                .unwrap();
        }
    }
}

pub fn inverse_kinematics_system(engine: &mut Engine) {
    let world = &mut engine.world;
    let input_context = &engine.input_context;
    let root_in_stage = {
        let hmd_in_stage = input_context.hmd.hmd_in_stage();
        let mut root_in_stage = hmd_in_stage;
        root_in_stage.translation.y = 0.0;
        let x_dir_in_stage = vec3a(
            root_in_stage.matrix3.x_axis.x,
            0.0,
            root_in_stage.matrix3.x_axis.z,
        )
        .normalize();
        root_in_stage.matrix3.x_axis = x_dir_in_stage;
        root_in_stage.matrix3.y_axis = Vec3A::Y;
        root_in_stage.matrix3.z_axis = x_dir_in_stage.cross(Vec3A::Y);
        root_in_stage
    };

    for (_, (local_transform, node)) in world
        .query_mut::<(&mut LocalTransform, &IkNode)>()
        .into_iter()
    {
        local_transform.update_from_affine(&match node.node_id {
            IkNodeID::LeftGrip => input_context.left.stage_from_grip(),
            IkNodeID::LeftAim => input_context.left.stage_from_aim(),
            IkNodeID::RightGrip => input_context.right.stage_from_grip(),
            IkNodeID::RightAim => input_context.right.stage_from_aim(),
            IkNodeID::Hmd => input_context.hmd.hmd_in_stage(),
            IkNodeID::Root => root_in_stage,
        });
    }
}
