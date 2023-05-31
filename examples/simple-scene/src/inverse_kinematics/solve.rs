use std::f32::consts::{FRAC_1_SQRT_2, FRAC_PI_2, FRAC_PI_4, PI, TAU};

use hotham::glam::{vec3, vec3a, Affine3A, Quat, Vec2, Vec3, Vec3A, Vec4};
use inline_tweak::tweak;

use crate::utils::lerp32;

use super::{
    constraints::*, left_thumbstick_influence, right_thumbstick_influence, set_ik_node_from_affine,
    BodyParameters, IkNodeID, IkState, WeightDistribution,
};

pub fn get_body_parameters() -> BodyParameters {
    BodyParameters {
        lower_arm_length: tweak!(0.28),
        upper_arm_length: tweak!(0.28),
        collarbone_length: tweak!(0.17),
        shoulder_width: tweak!(0.40),
        sternum_width: tweak!(0.06),
        hip_width: tweak!(0.26),
        sternum_height_in_torso: tweak!(0.20),
        neck_root_height_in_torso: tweak!(0.22),
        lower_back_height_in_torso: tweak!(-0.20),
        lower_back_height_in_pelvis: tweak!(0.10),
        hip_height_in_pelvis: tweak!(-0.07),
        upper_leg_length: tweak!(0.40),
        lower_leg_length: tweak!(0.40),
        ankle_height: tweak!(0.10),
        foot_height: tweak!(0.05),
    }
}

pub fn solve_ik(
    hmd_in_stage: Affine3A,
    left_grip_in_stage: Affine3A,
    left_aim_in_stage: Affine3A,
    right_grip_in_stage: Affine3A,
    right_aim_in_stage: Affine3A,
    left_thumbstick: Vec2,
    right_thumbstick: Vec2,
    body_parameters: &BodyParameters,
    state: &mut IkState,
) {
    puffin::profile_function!();
    let lower_arm_length = body_parameters.lower_arm_length;
    let upper_arm_length = body_parameters.upper_arm_length;
    let collarbone_length = body_parameters.collarbone_length;
    let shoulder_width = body_parameters.shoulder_width;
    let sternum_width = body_parameters.sternum_width;
    let hip_width = body_parameters.hip_width;
    let sternum_height_in_torso = body_parameters.sternum_height_in_torso;
    let neck_root_height_in_torso = body_parameters.neck_root_height_in_torso;
    let lower_back_height_in_torso = body_parameters.lower_back_height_in_torso;
    let lower_back_height_in_pelvis = body_parameters.lower_back_height_in_pelvis;
    let hip_height_in_pelvis = body_parameters.hip_height_in_pelvis;
    let upper_leg_length = body_parameters.upper_leg_length;
    let lower_leg_length = body_parameters.lower_leg_length;
    let ankle_height = body_parameters.ankle_height;
    let foot_height = body_parameters.foot_height;

    // Fixed transforms and parameters
    let head_center_in_hmd = Affine3A::from_translation(vec3(0.0, tweak!(0.0), tweak!(0.10)));
    let neck_root_in_head_center = Affine3A::from_translation(vec3(0.0, tweak!(-0.1), tweak!(0.0)));
    let left_wrist_in_palm =
        Affine3A::from_translation(vec3(tweak!(-0.015), tweak!(-0.01), tweak!(0.065)));
    let right_wrist_in_palm =
        Affine3A::from_translation((left_wrist_in_palm.translation * vec3a(-1.0, 1.0, 1.0)).into());
    let wrist_in_lower_arm = vec3a(0.0, 0.0, -lower_arm_length / 2.0);
    let elbow_in_lower_arm = vec3a(0.0, 0.0, lower_arm_length / 2.0);
    let elbow_in_upper_arm = vec3a(0.0, 0.0, -upper_arm_length / 2.0);
    let shoulder_in_upper_arm = vec3a(0.0, 0.0, upper_arm_length / 2.0);
    let left_shoulder_in_torso = vec3a(-shoulder_width / 2.0, sternum_height_in_torso, 0.0);
    let right_shoulder_in_torso = vec3a(shoulder_width / 2.0, sternum_height_in_torso, 0.0);
    let left_sc_joint_in_torso = vec3a(-sternum_width / 2.0, sternum_height_in_torso, 0.0);
    let right_sc_joint_in_torso = vec3a(sternum_width / 2.0, sternum_height_in_torso, 0.0);
    let neck_root_in_torso = vec3a(0.0, neck_root_height_in_torso, 0.0);
    let lower_back_in_torso = vec3a(0.0, lower_back_height_in_torso, 0.0);
    let lower_back_in_pelvis = vec3a(0.0, lower_back_height_in_pelvis, 0.0);
    let left_hip_in_pelvis = vec3a(-hip_width / 2.0, hip_height_in_pelvis, 0.0);
    let right_hip_in_pelvis = vec3a(hip_width / 2.0, hip_height_in_pelvis, 0.0);
    let hip_in_upper_leg = vec3a(0.0, upper_leg_length / 2.0, 0.0);
    let knee_in_upper_leg = vec3a(0.0, -upper_leg_length / 2.0, 0.0);
    let knee_in_lower_leg = vec3a(0.0, lower_leg_length / 2.0, 0.0);
    let ankle_in_lower_leg = vec3a(0.0, -lower_leg_length / 2.0, 0.0);
    let ankle_in_foot = vec3a(0.0, ankle_height - foot_height / 2.0, tweak!(0.05));
    let foot_target_in_foot = vec3a(0.0, -foot_height / 2.0, tweak!(0.0));
    let foot_radius = tweak!(0.1);
    let step_multiplier = tweak!(3.0);
    let step_size = foot_radius * (step_multiplier + 1.0);
    let stagger_threshold = foot_radius * tweak!(2.0);

    let shoulder_compliance = tweak!(25.0);
    let elbow_fixed_angle_compliance = tweak!(100000.0);
    let lower_back_compliance = tweak!(1000.0);
    let hip_fixed_angle_compliance = tweak!(10000.0);
    let knee_fixed_angle_compliance = tweak!(10000.0);
    let ankle_fixed_angle_compliance = tweak!(1000.0);
    let head_fixed_angle_compliance = tweak!(1000.0);
    let wrist_fixed_angle_compliance = tweak!(1000.0);

    let anchor_strength = tweak!(0.25);
    let knee_strength = tweak!(0.1);

    let spherical_constraints = [
        SphericalConstraint {
            // Left wrist
            node_a: IkNodeID::LeftPalm,
            node_b: IkNodeID::LeftLowerArm,
            point_in_a: left_wrist_in_palm.translation,
            point_in_b: wrist_in_lower_arm,
        },
        SphericalConstraint {
            // Right wrist
            node_a: IkNodeID::RightPalm,
            node_b: IkNodeID::RightLowerArm,
            point_in_a: right_wrist_in_palm.translation,
            point_in_b: wrist_in_lower_arm,
        },
        SphericalConstraint {
            // Left elbow
            node_a: IkNodeID::LeftLowerArm,
            node_b: IkNodeID::LeftUpperArm,
            point_in_a: elbow_in_lower_arm,
            point_in_b: elbow_in_upper_arm,
        },
        SphericalConstraint {
            // Right elbow
            node_a: IkNodeID::RightLowerArm,
            node_b: IkNodeID::RightUpperArm,
            point_in_a: elbow_in_lower_arm,
            point_in_b: elbow_in_upper_arm,
        },
        SphericalConstraint {
            // Neck
            node_a: IkNodeID::HeadCenter,
            node_b: IkNodeID::Torso,
            point_in_a: neck_root_in_head_center.translation,
            point_in_b: neck_root_in_torso,
        },
        SphericalConstraint {
            // Lower back
            node_a: IkNodeID::Torso,
            node_b: IkNodeID::Pelvis,
            point_in_a: lower_back_in_torso,
            point_in_b: lower_back_in_pelvis,
        },
        SphericalConstraint {
            // Left hip joint
            node_a: IkNodeID::Pelvis,
            node_b: IkNodeID::LeftUpperLeg,
            point_in_a: left_hip_in_pelvis,
            point_in_b: hip_in_upper_leg,
        },
        SphericalConstraint {
            // Right hip joint
            node_a: IkNodeID::Pelvis,
            node_b: IkNodeID::RightUpperLeg,
            point_in_a: right_hip_in_pelvis,
            point_in_b: hip_in_upper_leg,
        },
        SphericalConstraint {
            // Left knee
            node_a: IkNodeID::LeftUpperLeg,
            node_b: IkNodeID::LeftLowerLeg,
            point_in_a: knee_in_upper_leg,
            point_in_b: knee_in_lower_leg,
        },
        SphericalConstraint {
            // Right knee
            node_a: IkNodeID::RightUpperLeg,
            node_b: IkNodeID::RightLowerLeg,
            point_in_a: knee_in_upper_leg,
            point_in_b: knee_in_lower_leg,
        },
        SphericalConstraint {
            // Left ankle
            node_a: IkNodeID::LeftLowerLeg,
            node_b: IkNodeID::LeftFoot,
            point_in_a: ankle_in_lower_leg,
            point_in_b: ankle_in_foot,
        },
        SphericalConstraint {
            // Right ankle
            node_a: IkNodeID::RightLowerLeg,
            node_b: IkNodeID::RightFoot,
            point_in_a: ankle_in_lower_leg,
            point_in_b: ankle_in_foot,
        },
    ];
    let distance_constraints = [
        DistanceConstraint {
            // Left collarbone
            node_a: IkNodeID::LeftUpperArm,
            node_b: IkNodeID::Torso,
            point_in_a: shoulder_in_upper_arm,
            point_in_b: left_sc_joint_in_torso,
            distance: collarbone_length,
        },
        DistanceConstraint {
            // Right collarbone
            node_a: IkNodeID::RightUpperArm,
            node_b: IkNodeID::Torso,
            point_in_a: shoulder_in_upper_arm,
            point_in_b: right_sc_joint_in_torso,
            distance: collarbone_length,
        },
    ];
    let cardan_constraints = [
        AngularCardanConstraint {
            // Left knee
            node_a: IkNodeID::LeftUpperLeg,
            node_b: IkNodeID::LeftLowerLeg,
            axis_in_a: Vec3A::X,
            axis_in_b: Vec3A::Y,
        },
        AngularCardanConstraint {
            // Right knee
            node_a: IkNodeID::RightUpperLeg,
            node_b: IkNodeID::RightLowerLeg,
            axis_in_a: Vec3A::X,
            axis_in_b: Vec3A::Y,
        },
        AngularCardanConstraint {
            // Left ankle
            node_a: IkNodeID::LeftLowerLeg,
            node_b: IkNodeID::LeftFoot,
            axis_in_a: Vec3A::X,
            axis_in_b: vec3a(0.0, FRAC_1_SQRT_2, FRAC_1_SQRT_2),
        },
        AngularCardanConstraint {
            // Right ankle
            node_a: IkNodeID::RightLowerLeg,
            node_b: IkNodeID::RightFoot,
            axis_in_a: Vec3A::X,
            axis_in_b: vec3a(0.0, FRAC_1_SQRT_2, FRAC_1_SQRT_2),
        },
        AngularCardanConstraint {
            // Left elbow
            node_a: IkNodeID::LeftLowerArm,
            node_b: IkNodeID::LeftUpperArm,
            axis_in_a: Vec3A::Z,
            axis_in_b: Vec3A::X,
        },
        AngularCardanConstraint {
            // Right elbow
            node_a: IkNodeID::RightLowerArm,
            node_b: IkNodeID::RightUpperArm,
            axis_in_a: Vec3A::Z,
            axis_in_b: Vec3A::X,
        },
        AngularCardanConstraint {
            // Left wrist
            node_a: IkNodeID::LeftPalm,
            node_b: IkNodeID::LeftLowerArm,
            axis_in_a: Vec3A::X,
            axis_in_b: Vec3A::Y,
        },
        AngularCardanConstraint {
            // Right wrist
            node_a: IkNodeID::RightPalm,
            node_b: IkNodeID::RightLowerArm,
            axis_in_a: Vec3A::X,
            axis_in_b: Vec3A::Y,
        },
    ];
    let compliant_spherical_constraints = [
        CompliantSphericalConstraint {
            // Left shoulder
            node_a: IkNodeID::Torso,
            node_b: IkNodeID::LeftUpperArm,
            point_in_a: left_shoulder_in_torso,
            point_in_b: shoulder_in_upper_arm,
            compliance: shoulder_compliance,
        },
        CompliantSphericalConstraint {
            // Right shoulder
            node_a: IkNodeID::Torso,
            node_b: IkNodeID::RightUpperArm,
            point_in_a: right_shoulder_in_torso,
            point_in_b: shoulder_in_upper_arm,
            compliance: shoulder_compliance,
        },
    ];
    let compliant_fixed_angle_constraints = [
        CompliantFixedAngleConstraint {
            // Left wrist
            node_a: IkNodeID::LeftLowerArm,
            node_b: IkNodeID::LeftPalm,
            b_in_a: Quat::IDENTITY,
            compliance: wrist_fixed_angle_compliance,
        },
        CompliantFixedAngleConstraint {
            // Right wrist
            node_a: IkNodeID::RightLowerArm,
            node_b: IkNodeID::RightPalm,
            b_in_a: Quat::IDENTITY,
            compliance: wrist_fixed_angle_compliance,
        },
        CompliantFixedAngleConstraint {
            // Left ankle
            node_a: IkNodeID::LeftLowerLeg,
            node_b: IkNodeID::LeftFoot,
            b_in_a: Quat::IDENTITY,
            compliance: ankle_fixed_angle_compliance,
        },
        CompliantFixedAngleConstraint {
            // Right ankle
            node_a: IkNodeID::RightLowerLeg,
            node_b: IkNodeID::RightFoot,
            b_in_a: Quat::IDENTITY,
            compliance: ankle_fixed_angle_compliance,
        },
        CompliantFixedAngleConstraint {
            // Left knee
            node_a: IkNodeID::LeftUpperLeg,
            node_b: IkNodeID::LeftLowerLeg,
            b_in_a: Quat::from_axis_angle(Vec3::X, -FRAC_PI_2),
            compliance: knee_fixed_angle_compliance,
        },
        CompliantFixedAngleConstraint {
            // Right knee
            node_a: IkNodeID::RightUpperLeg,
            node_b: IkNodeID::RightLowerLeg,
            b_in_a: Quat::from_axis_angle(Vec3::X, -FRAC_PI_2),
            compliance: knee_fixed_angle_compliance,
        },
        CompliantFixedAngleConstraint {
            // Left elbow
            node_a: IkNodeID::LeftUpperArm,
            node_b: IkNodeID::LeftLowerArm,
            b_in_a: Quat::from_axis_angle(Vec3::X, FRAC_PI_2),
            compliance: elbow_fixed_angle_compliance,
        },
        CompliantFixedAngleConstraint {
            // Right elbow
            node_a: IkNodeID::RightUpperArm,
            node_b: IkNodeID::RightLowerArm,
            b_in_a: Quat::from_axis_angle(Vec3::X, FRAC_PI_2),
            compliance: elbow_fixed_angle_compliance,
        },
        CompliantFixedAngleConstraint {
            // Lower back
            node_a: IkNodeID::Torso,
            node_b: IkNodeID::Pelvis,
            b_in_a: Quat::IDENTITY,
            compliance: lower_back_compliance,
        },
        CompliantFixedAngleConstraint {
            // Left hip
            node_a: IkNodeID::Pelvis,
            node_b: IkNodeID::LeftUpperLeg,
            b_in_a: Quat::from_axis_angle(Vec3::X, FRAC_PI_4),
            compliance: hip_fixed_angle_compliance,
        },
        CompliantFixedAngleConstraint {
            // Right hip
            node_a: IkNodeID::Pelvis,
            node_b: IkNodeID::RightUpperLeg,
            b_in_a: Quat::from_axis_angle(Vec3::X, FRAC_PI_4),
            compliance: hip_fixed_angle_compliance,
        },
        CompliantFixedAngleConstraint {
            // Head
            node_a: IkNodeID::HeadCenter,
            node_b: IkNodeID::Torso,
            b_in_a: Quat::IDENTITY,
            compliance: head_fixed_angle_compliance,
        },
    ];
    let compliant_hinge_angle_constraints: [CompliantHingeAngleConstraint; 0] = [];

    // Dynamic transforms
    let head_center_in_stage = hmd_in_stage * head_center_in_hmd;
    let neck_root_in_stage = head_center_in_stage * neck_root_in_head_center;
    let base_in_stage = {
        let mut base_in_stage = neck_root_in_stage;
        base_in_stage.translation.y = 0.0;
        let x_dir_in_stage = vec3a(
            base_in_stage.matrix3.x_axis.x,
            0.0,
            base_in_stage.matrix3.x_axis.z,
        )
        .normalize();
        base_in_stage.matrix3.x_axis = x_dir_in_stage;
        base_in_stage.matrix3.y_axis = Vec3A::Y;
        base_in_stage.matrix3.z_axis = x_dir_in_stage.cross(Vec3A::Y);
        base_in_stage
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

    let left_foot_in_stage = state
        .left_foot_in_stage
        .unwrap_or_else(|| base_in_stage * Affine3A::from_translation(vec3(-0.2, 0.0, 0.0)));
    let right_foot_in_stage = state
        .right_foot_in_stage
        .unwrap_or_else(|| base_in_stage * Affine3A::from_translation(vec3(0.2, 0.0, 0.0)));
    let base_from_stage = base_in_stage.inverse();
    let left_foot_in_base = (base_from_stage * left_foot_in_stage).translation;
    let right_foot_in_base = (base_from_stage * right_foot_in_stage).translation;
    state.weight_distribution = match (
        left_foot_in_base.length() < foot_radius,
        right_foot_in_base.length() < foot_radius,
    ) {
        (true, true) => state.weight_distribution,
        (true, false) => WeightDistribution::LeftPlanted,
        (false, true) => WeightDistribution::RightPlanted,
        (false, false) => WeightDistribution::SharedWeight,
    };
    let balance_point_in_base = {
        let a = left_foot_in_base;
        let b = right_foot_in_base;
        let c = Vec3A::ZERO;
        let v = b - a;
        let t = (c - a).dot(v) / v.dot(v);
        a + v * t.clamp(0.0, 1.0)
    };
    match state.weight_distribution {
        WeightDistribution::RightPlanted => {
            state.left_foot_in_stage = Some(
                base_in_stage
                    * Affine3A::from_translation(vec3(
                        -step_multiplier * right_foot_in_base.x,
                        -step_multiplier * right_foot_in_base.y,
                        -step_multiplier * right_foot_in_base.z,
                    )),
            );
            state.right_foot_in_stage = Some(right_foot_in_stage);
        }
        WeightDistribution::LeftPlanted => {
            state.left_foot_in_stage = Some(left_foot_in_stage);
            state.right_foot_in_stage = Some(
                base_in_stage
                    * Affine3A::from_translation(vec3(
                        -step_multiplier * left_foot_in_base.x,
                        -step_multiplier * left_foot_in_base.y,
                        -step_multiplier * left_foot_in_base.z,
                    )),
            );
        }
        WeightDistribution::SharedWeight => {
            if balance_point_in_base.length() > stagger_threshold {
                // Stagger step, lift the foot that is loaded the least.
                let v1 = balance_point_in_base - left_foot_in_base;
                let v2 = balance_point_in_base - right_foot_in_base;
                if v1.length_squared() < v2.length_squared() {
                    let dir = -left_foot_in_base.normalize();
                    state.left_foot_in_stage = Some(left_foot_in_stage);
                    state.right_foot_in_stage = Some(
                        base_in_stage
                            * Affine3A::from_translation(
                                (left_foot_in_base + dir * step_size).into(),
                            ),
                    );
                } else {
                    let dir = -right_foot_in_base.normalize();
                    state.left_foot_in_stage = Some(
                        base_in_stage
                            * Affine3A::from_translation(
                                (right_foot_in_base + dir * step_size).into(),
                            ),
                    );
                    state.right_foot_in_stage = Some(right_foot_in_stage);
                }
            } else {
                state.left_foot_in_stage = Some(left_foot_in_stage);
                state.right_foot_in_stage = Some(right_foot_in_stage);
            }
        }
    }

    let hand_action_max_angle: f32 = std::f32::consts::FRAC_PI_2 * tweak!(0.5);
    let foot_action_kick_angle: f32 = std::f32::consts::FRAC_PI_2 * tweak!(1.0);
    let foot_action_knee_angle: f32 = std::f32::consts::FRAC_PI_2 * tweak!(-0.5);
    let left_action_blend = left_thumbstick_influence(left_thumbstick);
    let right_action_blend = right_thumbstick_influence(right_thumbstick);
    let left_hand_angle = hand_action_max_angle * left_action_blend.hand.z;
    let right_hand_angle = hand_action_max_angle * right_action_blend.hand.z;
    let left_foot_angle = lerp32(
        foot_action_kick_angle,
        foot_action_knee_angle,
        left_action_blend.foot.z / (left_action_blend.foot.y + left_action_blend.foot.z).max(0.001),
    );
    let right_foot_angle = lerp32(
        foot_action_kick_angle,
        foot_action_knee_angle,
        right_action_blend.foot.z
            / (right_action_blend.foot.y + right_action_blend.foot.z).max(0.001),
    );
    let left_hand_action_in_palm = Affine3A::from_rotation_x(left_hand_angle);
    let right_hand_action_in_palm = Affine3A::from_rotation_x(right_hand_angle);
    let left_foot_action_in_palm = Affine3A::from_translation(vec3(0.0, tweak!(0.0), tweak!(-0.1)))
        * Affine3A::from_rotation_x(left_foot_angle)
        * Affine3A::from_translation(vec3(0.0, tweak!(-0.5), tweak!(0.0)));
    let right_foot_action_in_palm =
        Affine3A::from_translation(vec3(0.0, tweak!(0.0), tweak!(-0.1)))
            * Affine3A::from_rotation_x(right_foot_angle)
            * Affine3A::from_translation(vec3(0.0, tweak!(-0.5), tweak!(0.0)));

    let left_hand_action_in_stage = left_palm_in_stage * left_hand_action_in_palm;
    let right_hand_action_in_stage = right_palm_in_stage * right_hand_action_in_palm;
    let left_foot_action_in_stage = left_palm_in_stage * left_foot_action_in_palm;
    let right_foot_action_in_stage = right_palm_in_stage * right_foot_action_in_palm;

    let anchor_constraints = [
        AnchorConstraint::from_affine(
            // Neutral
            IkNodeID::LeftPalm,
            &Vec3A::ZERO,
            &left_hand_action_in_stage,
            anchor_strength * left_action_blend.hand.x,
        ),
        AnchorConstraint::from_affine(
            // Neutral
            IkNodeID::RightPalm,
            &Vec3A::ZERO,
            &right_hand_action_in_stage,
            anchor_strength * right_action_blend.hand.x,
        ),
        AnchorConstraint::from_affine(
            // Punch
            IkNodeID::LeftLowerArm,
            &vec3a(0.0, 0.0, lower_arm_length / 2.0),
            &left_hand_action_in_stage,
            anchor_strength * left_action_blend.hand.y,
        ),
        AnchorConstraint::from_affine(
            // Punch
            IkNodeID::RightLowerArm,
            &vec3a(0.0, 0.0, lower_arm_length / 2.0),
            &right_hand_action_in_stage,
            anchor_strength * right_action_blend.hand.y,
        ),
        AnchorConstraint::from_affine(
            // Elbow
            IkNodeID::LeftLowerArm,
            &vec3a(0.0, 0.0, lower_arm_length / 2.0),
            &left_hand_action_in_stage,
            anchor_strength * left_action_blend.hand.z,
        ),
        AnchorConstraint::from_affine(
            // Elbow
            IkNodeID::RightLowerArm,
            &vec3a(0.0, 0.0, lower_arm_length / 2.0),
            &right_hand_action_in_stage,
            anchor_strength * right_action_blend.hand.z,
        ),
        AnchorConstraint::from_affine(
            // Neutral
            IkNodeID::LeftFoot,
            &foot_target_in_foot,
            &left_foot_in_stage,
            anchor_strength * left_action_blend.foot.x,
        ),
        AnchorConstraint::from_affine(
            // Neutral
            IkNodeID::RightFoot,
            &foot_target_in_foot,
            &right_foot_in_stage,
            anchor_strength * right_action_blend.foot.x,
        ),
        AnchorConstraint::from_affine(
            // Kick
            IkNodeID::LeftFoot,
            &foot_target_in_foot,
            &left_foot_action_in_stage,
            anchor_strength * left_action_blend.foot.y,
        ),
        AnchorConstraint::from_affine(
            // Kick
            IkNodeID::RightFoot,
            &foot_target_in_foot,
            &right_foot_action_in_stage,
            anchor_strength * right_action_blend.foot.y,
        ),
        AnchorConstraint::from_affine(
            // Knee
            IkNodeID::LeftFoot,
            &foot_target_in_foot,
            &left_foot_action_in_stage,
            anchor_strength * left_action_blend.foot.z,
        ),
        AnchorConstraint::from_affine(
            // Knee
            IkNodeID::RightFoot,
            &foot_target_in_foot,
            &right_foot_action_in_stage,
            anchor_strength * right_action_blend.foot.z,
        ),
        AnchorConstraint::from_affine(
            // Knee
            IkNodeID::LeftLowerLeg,
            &vec3a(0.0, 0.0, lower_leg_length / 2.0),
            &left_foot_action_in_stage,
            anchor_strength * left_action_blend.foot.z.powi(2) * knee_strength,
        ),
        AnchorConstraint::from_affine(
            // Knee
            IkNodeID::RightLowerLeg,
            &vec3a(0.0, 0.0, lower_leg_length / 2.0),
            &right_foot_action_in_stage,
            anchor_strength * right_action_blend.foot.z.powi(2) * knee_strength,
        ),
        AnchorConstraint::from_affine(
            IkNodeID::HeadCenter,
            &Vec3A::ZERO,
            &head_center_in_stage,
            anchor_strength * 1.0,
        ),
    ];

    // Update input nodes
    let fixed_nodes = [
        (IkNodeID::Hmd, hmd_in_stage),
        (IkNodeID::LeftGrip, left_grip_in_stage),
        (IkNodeID::LeftAim, left_aim_in_stage),
        (IkNodeID::RightGrip, right_grip_in_stage),
        (IkNodeID::RightAim, right_aim_in_stage),
        // Helpers
        (IkNodeID::NeckRoot, neck_root_in_stage),
        (IkNodeID::Base, base_in_stage),
        (
            IkNodeID::BalancePoint,
            base_in_stage * Affine3A::from_translation(balance_point_in_base.into()),
        ),
        (IkNodeID::LeftFootTarget, left_foot_in_stage),
        (IkNodeID::RightFootTarget, right_foot_in_stage),
        (IkNodeID::LeftWrist, left_wrist_in_stage),
        (IkNodeID::RightWrist, right_wrist_in_stage),
    ];
    for (node_id, node_in_stage) in fixed_nodes.iter() {
        set_ik_node_from_affine(state, node_id, node_in_stage);
    }
    // Solve IK
    puffin::profile_scope!("solve ik iterations");
    for _ in 0..tweak!(100) {
        puffin::profile_scope!("solve ik iteration");
        for constraint in &anchor_constraints {
            let node_a = constraint.node_a as usize;
            let r1 = state.node_rotations[node_a] * constraint.point_in_a;
            // w = inv_mass + p.cross(n)ᵀ * inv_inertia * p.cross(n)
            let r1_squares = r1 * r1;
            let w1 = vec3a(
                1.0 + r1_squares.y + r1_squares.z,
                1.0 + r1_squares.z + r1_squares.x,
                1.0 + r1_squares.x + r1_squares.y,
            );
            let p1 = state.node_positions[node_a] + r1;
            let p2 = constraint.target_point_in_stage;
            let c = p1 - p2;
            let correction = constraint.strength * -c / w1;
            state.node_positions[node_a] += correction;
            // q1 <- q1 + 0.5 * (p1.cross(correction) * q1)
            let q1 = &mut state.node_rotations[node_a];
            let omega = r1.cross(correction);
            *q1 = Quat::from_vec4(
                Vec4::from(*q1) + 0.5 * Vec4::from(Quat::from_vec4(omega.extend(0.0)) * *q1),
            )
            .normalize();

            // Rotational part
            state.node_rotations[node_a] = state.node_rotations[node_a]
                .lerp(constraint.target_rot_in_stage, constraint.strength);
        }
        for constraint in &spherical_constraints {
            let node_a = constraint.node_a as usize;
            let node_b = constraint.node_b as usize;
            let r1 = state.node_rotations[node_a] * constraint.point_in_a;
            let r2 = state.node_rotations[node_b] * constraint.point_in_b;
            // w = inv_mass + p.cross(n)ᵀ * inv_inertia * p.cross(n)
            let r1_squares = r1 * r1;
            let w1 = vec3a(
                1.0 + r1_squares.y + r1_squares.z,
                1.0 + r1_squares.z + r1_squares.x,
                1.0 + r1_squares.x + r1_squares.y,
            );
            let r2_squares = r2 * r2;
            let w2 = vec3a(
                1.0 + r2_squares.y + r2_squares.z,
                1.0 + r2_squares.z + r2_squares.x,
                1.0 + r2_squares.x + r2_squares.y,
            );
            let p1 = state.node_positions[node_a] + r1;
            let p2 = state.node_positions[node_b] + r2;
            let c = p1 - p2;
            let correction = -c / (w1 + w2);
            state.node_positions[node_a] += correction;
            state.node_positions[node_b] -= correction;
            // q1 <- q1 + 0.5 * (p1.cross(correction) * q1)
            let q1 = &mut state.node_rotations[node_a];
            let omega = r1.cross(correction);
            *q1 = Quat::from_vec4(
                Vec4::from(*q1) + 0.5 * Vec4::from(Quat::from_vec4(omega.extend(0.0)) * *q1),
            )
            .normalize();
            // q2 <- q2 - 0.5 * (p1.cross(correction) * q2)
            let q2 = &mut state.node_rotations[node_b];
            let omega = r2.cross(correction);
            *q2 = Quat::from_vec4(
                Vec4::from(*q2) - 0.5 * Vec4::from(Quat::from_vec4(omega.extend(0.0)) * *q2),
            )
            .normalize();
        }
        for constraint in &distance_constraints {
            let node_a = constraint.node_a as usize;
            let node_b = constraint.node_b as usize;
            let r1 = state.node_rotations[node_a] * constraint.point_in_a;
            let r2 = state.node_rotations[node_b] * constraint.point_in_b;
            // w = inv_mass + p.cross(n)ᵀ * inv_inertia * p.cross(n)
            let r1_squares = r1 * r1;
            let w1 = vec3a(
                1.0 + r1_squares.y + r1_squares.z,
                1.0 + r1_squares.z + r1_squares.x,
                1.0 + r1_squares.x + r1_squares.y,
            );
            let r2_squares = r2 * r2;
            let w2 = vec3a(
                1.0 + r2_squares.y + r2_squares.z,
                1.0 + r2_squares.z + r2_squares.x,
                1.0 + r2_squares.x + r2_squares.y,
            );
            let p1 = state.node_positions[node_a] + r1;
            let p2 = state.node_positions[node_b] + r2;
            let v = p1 - p2;
            let v_length = v.length();
            let c = v_length - constraint.distance;
            let correction = (-c / ((w1 + w2) * v_length)) * v;
            state.node_positions[node_a] += correction;
            state.node_positions[node_b] -= correction;
            // q1 <- q1 + 0.5 * (p1.cross(correction) * q1)
            let q1 = &mut state.node_rotations[node_a];
            let omega = r1.cross(correction);
            *q1 = Quat::from_vec4(
                Vec4::from(*q1) + 0.5 * Vec4::from(Quat::from_vec4(omega.extend(0.0)) * *q1),
            )
            .normalize();
            // q2 <- q2 - 0.5 * (p1.cross(correction) * q2)
            let q2 = &mut state.node_rotations[node_b];
            let omega = r2.cross(correction);
            *q2 = Quat::from_vec4(
                Vec4::from(*q2) - 0.5 * Vec4::from(Quat::from_vec4(omega.extend(0.0)) * *q2),
            )
            .normalize();
        }
        for constraint in &cardan_constraints {
            let node_a = constraint.node_a as usize;
            let node_b = constraint.node_b as usize;
            let axis1 = state.node_rotations[node_a] * constraint.axis_in_a;
            let axis2 = state.node_rotations[node_b] * constraint.axis_in_b;
            // The constraint is satisfied when the axes are perpendicular
            let angle = axis1.dot(axis2).acos() - FRAC_PI_2;
            let axis = axis1.cross(axis2).normalize();
            // The correction is applied symmetrically
            let (s, c) = (angle * 0.25).sin_cos();
            let v = axis * s;
            let delta1 = Quat::from_xyzw(v.x, v.y, v.z, c);
            let delta2 = Quat::from_xyzw(-v.x, -v.y, -v.z, c);
            state.node_rotations[node_a] = delta1 * state.node_rotations[node_a];
            state.node_rotations[node_b] = delta2 * state.node_rotations[node_b];
        }
        for constraint in &compliant_spherical_constraints {
            let node_a = constraint.node_a as usize;
            let node_b = constraint.node_b as usize;
            let r1 = state.node_rotations[node_a] * constraint.point_in_a;
            let r2 = state.node_rotations[node_b] * constraint.point_in_b;
            // w = inv_mass + p.cross(n)ᵀ * inv_inertia * p.cross(n)
            let r1_squares = r1 * r1;
            let w1 = vec3a(
                1.0 + r1_squares.y + r1_squares.z,
                1.0 + r1_squares.z + r1_squares.x,
                1.0 + r1_squares.x + r1_squares.y,
            );
            let r2_squares = r2 * r2;
            let w2 = vec3a(
                1.0 + r2_squares.y + r2_squares.z,
                1.0 + r2_squares.z + r2_squares.x,
                1.0 + r2_squares.x + r2_squares.y,
            );
            let p1 = state.node_positions[node_a] + r1;
            let p2 = state.node_positions[node_b] + r2;
            let c = p1 - p2;
            let correction = -c / (w1 + w2 + constraint.compliance);
            state.node_positions[node_a] += correction;
            state.node_positions[node_b] -= correction;
            // q1 <- q1 + 0.5 * (p1.cross(correction) * q1)
            let q1 = &mut state.node_rotations[node_a];
            let omega = r1.cross(correction);
            *q1 = Quat::from_vec4(
                Vec4::from(*q1) + 0.5 * Vec4::from(Quat::from_vec4(omega.extend(0.0)) * *q1),
            )
            .normalize();
            // q2 <- q2 - 0.5 * (p1.cross(correction) * q2)
            let q2 = &mut state.node_rotations[node_b];
            let omega = r2.cross(correction);
            *q2 = Quat::from_vec4(
                Vec4::from(*q2) - 0.5 * Vec4::from(Quat::from_vec4(omega.extend(0.0)) * *q2),
            )
            .normalize();
        }
        for constraint in &compliant_fixed_angle_constraints {
            let node_a = constraint.node_a as usize;
            let node_b = constraint.node_b as usize;
            let stage_from_a = state.node_rotations[node_a];
            let stage_from_b = state.node_rotations[node_b];
            let stage_from_wanted_b = stage_from_a * constraint.b_in_a;
            let delta = stage_from_b * stage_from_wanted_b.inverse();
            let (axis, mut angle) = delta.to_axis_angle();
            if angle > PI {
                angle -= TAU;
            }
            let (s, c) = (angle * 0.5 / (2.0 + constraint.compliance)).sin_cos();
            let v = axis * s;
            let delta1 = Quat::from_xyzw(v.x, v.y, v.z, c);
            let delta2 = Quat::from_xyzw(-v.x, -v.y, -v.z, c);
            state.node_rotations[node_a] = delta1 * state.node_rotations[node_a];
            state.node_rotations[node_b] = delta2 * state.node_rotations[node_b];
        }
        for constraint in &compliant_hinge_angle_constraints {
            let node_a = constraint.node_a as usize;
            let node_b = constraint.node_b as usize;
            let axis1 = state.node_rotations[node_a] * constraint.axis_in_a;
            let axis2 = state.node_rotations[node_b] * constraint.axis_in_b;
            // The constraint is satisfied when the axes are aligned
            let omega = axis1.cross(axis2);
            let factor = 1.0 / (2.0 + constraint.compliance);
            // q1 <- q1 + 0.5 * (axis1.cross(axis2) * q1)
            let q1 = &mut state.node_rotations[node_a];
            *q1 = Quat::from_vec4(
                Vec4::from(*q1) + factor * Vec4::from(Quat::from_vec4(omega.extend(0.0)) * *q1),
            )
            .normalize();
            // q2 <- q2 - 0.5 * (axis1.cross(axis2) * q2)
            let q2 = &mut state.node_rotations[node_b];
            *q2 = Quat::from_vec4(
                Vec4::from(*q2) - factor * Vec4::from(Quat::from_vec4(omega.extend(0.0)) * *q2),
            )
            .normalize();
        }
    }
}

#[test]
fn test_cardan() {
    let axis1 = Vec3A::X;
    let axis2 = vec3a(0.5, 1.0, 0.2).normalize();

    let angle = axis1.dot(axis2).acos() - FRAC_PI_2;
    let axis = axis1.cross(axis2).normalize();
    let (s, c) = (angle * 0.25).sin_cos();
    let v = axis * s;
    let delta1 = Quat::from_xyzw(v.x, v.y, v.z, c);
    let delta2 = Quat::from_xyzw(-v.x, -v.y, -v.z, c);

    let axis1_after = delta1 * axis1;
    let axis2_after = delta2 * axis2;
    let scalar = axis1_after.dot(axis2_after);
    println!("{scalar}");
}
