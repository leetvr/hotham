mod constraints;
mod solve;
#[cfg(test)]
mod tests;

use solve::solve_ik;

use std::collections::HashMap;

use enum_iterator::{all, cardinality, Sequence};
use serde::{Deserialize, Serialize};

use hotham::{
    asset_importer::add_model_to_world,
    components::{physics::SharedShape, Collider, LocalTransform, Stage},
    glam::{Affine3A, Quat, Vec3A},
    hecs::World,
    Engine,
};

use crate::thumbstick_influence::{left_thumbstick_influence, right_thumbstick_influence};

#[cfg(test)]
mod rr {
    pub use rerun::{
        components::{Box3D, ColorRGBA, Quaternion, Radius, Rigid3, Transform, Vec3D},
        time::{Time, TimeType, Timeline},
        MsgSender, Session,
    };
}

#[derive(Copy, Clone, Eq, Hash, Debug, PartialEq, Sequence, Deserialize, Serialize)]
#[repr(u8)]
pub enum IkNodeID {
    // Inputs
    Hmd,
    LeftAim,
    LeftGrip,
    RightAim,
    RightGrip,
    // Helpers
    NeckRoot,
    LeftWrist,
    RightWrist,
    Base,
    BalancePoint,
    LeftFootTarget,
    RightFootTarget,
    // Body
    HeadCenter,
    Torso,
    Pelvis,
    LeftPalm,
    LeftLowerArm,
    LeftUpperArm,
    LeftLowerLeg,
    LeftUpperLeg,
    LeftFoot,
    RightPalm,
    RightLowerArm,
    RightUpperArm,
    RightLowerLeg,
    RightUpperLeg,
    RightFoot,
}

pub struct IkNode {
    node_id: IkNodeID,
}

#[derive(Default)]
pub struct IkState {
    pub left_foot_in_stage: Option<Affine3A>,
    pub right_foot_in_stage: Option<Affine3A>,
    pub weight_distribution: WeightDistribution,
    pub balance_offset: Vec3A,
    pub node_positions: [Vec3A; cardinality::<IkNodeID>()],
    pub node_rotations: [Quat; cardinality::<IkNodeID>()],
}

#[derive(Clone, Copy)]
pub enum WeightDistribution {
    LeftPlanted,
    RightPlanted,
    SharedWeight,
}

impl Default for WeightDistribution {
    fn default() -> Self {
        Self::SharedWeight
    }
}

pub fn add_ik_nodes(models: &std::collections::HashMap<String, World>, world: &mut World) {
    let collider = Collider::new(SharedShape::ball(0.1));
    for node_id in all::<IkNodeID>() {
        let entity =
            add_model_to_world(model_name_from_node_id(node_id), models, world, None).unwrap();
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
            let entity = add_model_to_world(
                model_name_from_node_id(node_id),
                models,
                world,
                Some(parent),
            )
            .unwrap();
            world
                .insert(entity, (collider.clone(), IkNode { node_id }))
                .unwrap();
        }
    }
}

fn model_name_from_node_id(node_id: IkNodeID) -> &'static str {
    match node_id {
        // Inputs
        IkNodeID::Hmd => "Axes",
        IkNodeID::LeftGrip | IkNodeID::RightGrip => "SmallAxes",
        IkNodeID::LeftAim | IkNodeID::RightAim => "TinyAxes",
        // Helpers
        IkNodeID::NeckRoot => "SmallAxes",
        IkNodeID::Base => "Axes",
        IkNodeID::BalancePoint => "SmallAxes",
        IkNodeID::LeftWrist | IkNodeID::RightWrist => "CrossAxes",
        IkNodeID::LeftFootTarget | IkNodeID::RightFootTarget => "DiscXZ",
        // Body
        IkNodeID::HeadCenter => "Head",
        IkNodeID::Torso => "Torso",
        IkNodeID::Pelvis => "Pelvis",
        IkNodeID::LeftPalm => "LeftPalm",
        IkNodeID::LeftLowerArm => "LeftLowerArm",
        IkNodeID::LeftUpperArm => "LeftUpperArm",
        IkNodeID::LeftUpperLeg => "LeftUpperLeg",
        IkNodeID::LeftLowerLeg => "LeftLowerLeg",
        IkNodeID::RightPalm => "RightPalm",
        IkNodeID::RightLowerArm => "RightLowerArm",
        IkNodeID::RightUpperArm => "RightUpperArm",
        IkNodeID::RightUpperLeg => "RightUpperLeg",
        IkNodeID::RightLowerLeg => "RightLowerLeg",
        IkNodeID::LeftFoot => "LeftFoot",
        IkNodeID::RightFoot => "RightFoot",
    }
}

pub fn inverse_kinematics_system(
    engine: &mut Engine,
    state: &mut IkState,
    #[cfg(test)] session: Option<&mut rr::Session>,
) {
    puffin::profile_function!();
    let world = &mut engine.world;
    let input_context = &engine.input_context;
    let (shoulder_width, hip_width, sternum_height_in_torso, hip_height_in_pelvis) = solve_ik(
        input_context.hmd.hmd_in_stage(),
        input_context.left.stage_from_grip(),
        input_context.left.stage_from_aim(),
        input_context.right.stage_from_grip(),
        input_context.right.stage_from_aim(),
        input_context.left.thumbstick_xy(),
        input_context.right.thumbstick_xy(),
        state,
    );

    // Update entity transforms
    for (_, (local_transform, node)) in world
        .query_mut::<(&mut LocalTransform, &IkNode)>()
        .into_iter()
    {
        let node_id = node.node_id as usize;
        local_transform.translation = state.node_positions[node_id].into();
        local_transform.rotation = state.node_rotations[node_id];
    }

    // Store snapshot of current state if menu button is pressed
    if input_context.left.menu_button_just_pressed() {
        let date_time = chrono::Local::now().naive_local();
        let filename = date_time
            .format("inverse_kinematics_snapshot_%Y-%m-%d_%H.%M.%S.json")
            .to_string();
        println!("[INVERSE_KINEMATICS] Storing snapshot to '{filename}'");
        store_snapshot(state, &filename);
        let thumbsticks = (
            input_context.left.thumbstick_xy(),
            input_context.right.thumbstick_xy(),
        );
        println!("[INVERSE_KINEMATICS] Thumbsticks are {thumbsticks:?}");
    }

    // Send poses to rerun
    #[cfg(test)]
    if let Some(session) = session {
        send_poses_to_rerun(
            session,
            state,
            shoulder_width,
            sternum_height_in_torso,
            hip_width,
            hip_height_in_pelvis,
        );
    }
}

fn store_snapshot(state: &IkState, filename: &str) {
    let mut summary = HashMap::<IkNodeID, (Vec3A, Quat)>::new();
    for node_id in all::<IkNodeID>() {
        summary.insert(
            node_id,
            (
                state.node_positions[node_id as usize],
                state.node_rotations[node_id as usize],
            ),
        );
    }
    let serialized = serde_json::to_string(&summary).unwrap();
    std::fs::write(filename, serialized).expect(&format!("failed to write to '{filename}'"));
}

#[cfg(test)]
fn load_snapshot(state: &mut IkState, data: &str) {
    puffin::profile_function!();
    let summary: HashMap<IkNodeID, (Vec3A, Quat)> =
        serde_json::from_str(data).expect("JSON does not have correct format.");

    for node_id in all::<IkNodeID>() {
        if let Some((pos, rot)) = summary.get(&node_id) {
            state.node_positions[node_id as usize] = *pos;
            state.node_rotations[node_id as usize] = *rot;
        }
    }
}

#[cfg(test)]
fn load_snapshot_subset(state: &mut IkState, data: &str, subset: &[IkNodeID]) {
    let summary: HashMap<IkNodeID, (Vec3A, Quat)> =
        serde_json::from_str(data).expect("JSON does not have correct format.");

    for &node_id in subset {
        if let Some((pos, rot)) = summary.get(&node_id) {
            state.node_positions[node_id as usize] = *pos;
            state.node_rotations[node_id as usize] = *rot;
        }
    }
}

fn set_ik_node_from_affine(state: &mut IkState, node_id: &IkNodeID, node_in_stage: &Affine3A) {
    let (pos, rot) = to_pos_rot(node_in_stage);
    state.node_positions[*node_id as usize] = pos;
    state.node_rotations[*node_id as usize] = rot;
}

#[cfg(test)]
fn send_poses_to_rerun(
    #[cfg(test)] session: &rerun::Session,
    state: &IkState,
    shoulder_width: f32,
    sternum_height_in_torso: f32,
    hip_width: f32,
    hip_height_in_pelvis: f32,
) {
    puffin::profile_function!();
    let radius = rr::Radius(0.001);
    let log_fn = || -> hotham::anyhow::Result<()> {
        for node_id in all::<IkNodeID>() {
            let translation = &state.node_positions[node_id as usize];
            let rotation = &state.node_rotations[node_id as usize];
            let box_shape = match node_id {
                IkNodeID::HeadCenter => rr::Box3D::new(0.08, 0.11, 0.11),
                IkNodeID::Hmd => rr::Box3D::new(0.08, 0.04, 0.05),
                IkNodeID::LeftAim
                | IkNodeID::LeftGrip
                | IkNodeID::LeftWrist
                | IkNodeID::RightAim
                | IkNodeID::RightGrip
                | IkNodeID::RightWrist
                | IkNodeID::BalancePoint
                | IkNodeID::NeckRoot => rr::Box3D::new(0.01, 0.01, 0.01),
                IkNodeID::Torso => {
                    rr::Box3D::new(shoulder_width / 2.0, sternum_height_in_torso, 0.10)
                }
                IkNodeID::Pelvis => rr::Box3D::new(hip_width / 2.0, hip_height_in_pelvis, 0.10),
                IkNodeID::LeftFootTarget | IkNodeID::RightFootTarget | IkNodeID::Base => {
                    rr::Box3D::new(0.05, 0.001, 0.05)
                }
                IkNodeID::LeftPalm | IkNodeID::RightPalm => rr::Box3D::new(0.025, 0.05, 0.10),
                IkNodeID::LeftLowerArm
                | IkNodeID::LeftUpperArm
                | IkNodeID::RightLowerArm
                | IkNodeID::RightUpperArm => rr::Box3D::new(0.05, 0.05, 0.14),
                IkNodeID::LeftUpperLeg
                | IkNodeID::LeftLowerLeg
                | IkNodeID::RightUpperLeg
                | IkNodeID::RightLowerLeg => rr::Box3D::new(0.075, 0.20, 0.075),
                IkNodeID::LeftFoot | IkNodeID::RightFoot => rr::Box3D::new(0.05, 0.025, 0.14),
            };
            rr::MsgSender::new(format!("stage/{:?}", node_id))
                .with_component(&[rr::Transform::Rigid3(rr::Rigid3 {
                    rotation: rr::Quaternion {
                        w: rotation.w,
                        x: rotation.x,
                        y: rotation.y,
                        z: rotation.z,
                    },
                    translation: rr::Vec3D([translation.x, translation.y, translation.z]),
                })])?
                .with_splat(box_shape)?
                .with_splat(radius)?
                .send(session)?;
        }
        Ok(())
    };
    log_fn().unwrap_or_else(|e| {
        eprintln!("Failed to send poses to rerun: {e}");
    });
}

fn to_pos_rot(transform: &Affine3A) -> (Vec3A, Quat) {
    let (_scale, rotation, translation) = transform.to_scale_rotation_translation();
    (translation.into(), rotation)
}
