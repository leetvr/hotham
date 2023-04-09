use std::collections::HashMap;

use enum_iterator::{all, Sequence};
use serde::{Deserialize, Serialize};

use hotham::{
    asset_importer::add_model_to_world,
    components::{physics::SharedShape, Collider, LocalTransform, Stage},
    glam::{vec3, vec3a, Affine3A, Vec3, Vec3A},
    hecs::World,
    Engine,
};
use inline_tweak::tweak;

#[derive(Copy, Clone, Eq, Hash, Debug, PartialEq, Sequence, Deserialize, Serialize)]
pub enum IkNodeID {
    LeftGrip,
    LeftAim,
    LeftPalm,
    LeftWrist,
    RightGrip,
    RightAim,
    RightPalm,
    RightWrist,
    Hmd,
    HeadCenter,
    NeckRoot,
    Root,
    LeftFoot,
    RightFoot,
}

pub struct IkNode {
    node_id: IkNodeID,
}

#[derive(Default)]
pub struct IkState {
    pub left_foot_in_stage: Option<Affine3A>,
    pub right_foot_in_stage: Option<Affine3A>,
    pub floating_foot: LeftRightOrNone,
}

#[derive(Clone, Copy)]
pub enum LeftRightOrNone {
    Left,
    Right,
    None,
}

impl Default for LeftRightOrNone {
    fn default() -> Self {
        Self::None
    }
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

pub fn inverse_kinematics_system(engine: &mut Engine, state: &mut IkState) {
    // Fixed transforms
    let head_center_in_hmd = Affine3A::from_translation(vec3(0.0, tweak!(0.0), tweak!(0.10)));
    let neck_root_in_head_center = Affine3A::from_translation(vec3(0.0, tweak!(-0.1), tweak!(0.0)));
    let left_wrist_in_palm =
        Affine3A::from_translation(vec3(tweak!(-0.015), tweak!(-0.01), tweak!(0.065)));
    let right_wrist_in_palm =
        Affine3A::from_translation((left_wrist_in_palm.translation * vec3a(-1.0, 1.0, 1.0)).into());

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
    let left_wrist_in_stage = left_palm_in_stage * left_wrist_in_palm;
    let right_wrist_in_stage = right_palm_in_stage * right_wrist_in_palm;

    let foot_radius = tweak!(0.1);
    let left_foot_in_stage = state
        .left_foot_in_stage
        .unwrap_or_else(|| root_in_stage * Affine3A::from_translation(vec3(-0.2, 0.0, 0.0)));
    let right_foot_in_stage = state
        .right_foot_in_stage
        .unwrap_or_else(|| root_in_stage * Affine3A::from_translation(vec3(0.2, 0.0, 0.0)));
    let root_from_stage = root_in_stage.inverse();
    let left_foot_in_root = root_from_stage * left_foot_in_stage;
    let right_foot_in_root = root_from_stage * right_foot_in_stage;
    let step_multiplier = tweak!(3.0);
    state.floating_foot = match (
        left_foot_in_root.translation.length() < foot_radius,
        right_foot_in_root.translation.length() < foot_radius,
    ) {
        (true, true) => state.floating_foot,
        (true, false) => LeftRightOrNone::Right,
        (false, true) => LeftRightOrNone::Left,
        (false, false) => LeftRightOrNone::None,
    };
    match state.floating_foot {
        LeftRightOrNone::Left => {
            state.left_foot_in_stage = Some(
                root_in_stage
                    * Affine3A::from_translation(
                        Vec3::from(right_foot_in_root.translation)
                            * vec3(-step_multiplier, -step_multiplier, -step_multiplier),
                    ),
            );
            state.right_foot_in_stage = Some(right_foot_in_stage);
        }
        LeftRightOrNone::Right => {
            state.left_foot_in_stage = Some(left_foot_in_stage);
            state.right_foot_in_stage = Some(
                root_in_stage
                    * Affine3A::from_translation(
                        Vec3::from(left_foot_in_root.translation)
                            * vec3(-step_multiplier, -step_multiplier, -step_multiplier),
                    ),
            );
        }
        LeftRightOrNone::None => {
            state.left_foot_in_stage = Some(left_foot_in_stage);
            state.right_foot_in_stage = Some(right_foot_in_stage);
        }
    }

    // Update entity transforms
    let transform_of_node = |node_id: IkNodeID| match node_id {
        IkNodeID::LeftGrip => left_grip_in_stage,
        IkNodeID::LeftAim => left_aim_in_stage,
        IkNodeID::LeftPalm => left_palm_in_stage,
        IkNodeID::LeftWrist => left_wrist_in_stage,
        IkNodeID::RightGrip => right_grip_in_stage,
        IkNodeID::RightAim => right_aim_in_stage,
        IkNodeID::RightPalm => right_palm_in_stage,
        IkNodeID::RightWrist => right_wrist_in_stage,
        IkNodeID::Hmd => hmd_in_stage,
        IkNodeID::HeadCenter => head_center_in_stage,
        IkNodeID::NeckRoot => neck_root_in_stage,
        IkNodeID::Root => root_in_stage,
        IkNodeID::LeftFoot => state.left_foot_in_stage.unwrap(),
        IkNodeID::RightFoot => state.right_foot_in_stage.unwrap(),
    };
    for (_, (local_transform, node)) in world
        .query_mut::<(&mut LocalTransform, &IkNode)>()
        .into_iter()
    {
        local_transform.update_from_affine(&transform_of_node(node.node_id));
    }

    // Store snapshot of current state if menu button is pressed
    if input_context.left.menu_button_just_pressed() {
        let mut summary = HashMap::<IkNodeID, Affine3A>::new();
        for node_id in all::<IkNodeID>() {
            summary.insert(node_id, transform_of_node(node_id));
        }
        let serialized = serde_json::to_string(&summary).unwrap();
        let date_time = chrono::Local::now().naive_local();
        let filename = date_time
            .format("inverse_kinematics_snapshot_%Y-%m-%d_%H.%M.%S.json")
            .to_string();
        println!("[INVERSE_KINEMATICS] Storing snapshot to '{}'", filename);
        std::fs::write(&filename, serialized).expect(&format!("failed to write to '{filename}'"));
    }
}
