use enum_iterator::{all, Sequence};

use hotham::{
    asset_importer::add_model_to_world,
    components::{physics::SharedShape, Collider, LocalTransform, Stage},
    glam::{vec3, vec3a, Affine3A, Quat, Vec3, Vec3A},
    hecs::World,
    Engine,
};
use inline_tweak::tweak;

#[derive(Debug, PartialEq, Sequence)]
pub enum IkNodeID {
    // LeftGrip,
    // LeftAim,
    LeftPalm,
    // RightGrip,
    // RightAim,
    RightPalm,
    Hmd,
    HeadCenter,
    NeckRoot,
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
    // Fixed transforms
    let head_center_in_hmd = Affine3A::from_scale_rotation_translation(
        Vec3::ONE,
        Quat::IDENTITY,
        vec3(0.0, tweak!(0.0), tweak!(0.10)),
    );
    let neck_root_in_head_center = Affine3A::from_scale_rotation_translation(
        Vec3::ONE,
        Quat::IDENTITY,
        vec3(0.0, tweak!(-0.1), tweak!(0.0)),
    );

    // Dynamic transforms
    let world = &mut engine.world;
    let input_context = &engine.input_context;
    let hmd_in_stage = input_context.hmd.hmd_in_stage();
    let left_grip_in_stage = input_context.left.stage_from_grip();
    let left_aim_in_stage = input_context.left.stage_from_aim();
    let right_grip_in_stage = input_context.right.stage_from_grip();
    let right_aim_in_stage = input_context.right.stage_from_aim();
    let head_center_in_stage = hmd_in_stage * head_center_in_hmd;
    let neck_root_in_stage = head_center_in_stage * neck_root_in_head_center;
    let root_in_stage = {
        let mut root_in_stage = neck_root_in_stage;
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

    let left_palm_in_stage = {
        let mut left_palm_in_stage = left_grip_in_stage;
        left_palm_in_stage.matrix3 = left_aim_in_stage.matrix3;
        left_palm_in_stage
    };
    let right_palm_in_stage = {
        let mut right_palm_in_stage = right_grip_in_stage;
        right_palm_in_stage.matrix3 = right_aim_in_stage.matrix3;
        right_palm_in_stage
    };

    // Update entity transforms
    for (_, (local_transform, node)) in world
        .query_mut::<(&mut LocalTransform, &IkNode)>()
        .into_iter()
    {
        local_transform.update_from_affine(&match node.node_id {
            // IkNodeID::LeftGrip => left_grip_in_stage,
            // IkNodeID::LeftAim => left_aim_in_stage,
            IkNodeID::LeftPalm => left_palm_in_stage,
            // IkNodeID::RightGrip => right_grip_in_stage,
            // IkNodeID::RightAim => right_aim_in_stage,
            IkNodeID::RightPalm => right_palm_in_stage,
            IkNodeID::Hmd => hmd_in_stage,
            IkNodeID::HeadCenter => head_center_in_stage,
            IkNodeID::NeckRoot => neck_root_in_stage,
            IkNodeID::Root => root_in_stage,
        });
    }
}
