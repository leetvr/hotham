use hotham::glam::Affine3A;

use hotham::glam::Quat;

use hotham::glam::Vec3A;

use super::IkNodeID;

pub struct AnchorConstraint {
    pub node_a: IkNodeID,
    pub point_in_a: Vec3A,
    pub target_point_in_stage: Vec3A,
    pub target_rot_in_stage: Quat,
    pub strength: f32, // 0.0 to 1.0
}

impl AnchorConstraint {
    pub fn from_affine(
        node_a: IkNodeID,
        point_in_a: &Vec3A,
        target_in_stage: &Affine3A,
        strength: f32,
    ) -> Self {
        let (_scale, target_rot_in_stage, target_point_in_stage) =
            target_in_stage.to_scale_rotation_translation();
        Self {
            node_a,
            point_in_a: *point_in_a,
            target_point_in_stage: target_point_in_stage.into(),
            target_rot_in_stage,
            strength,
        }
    }
}

pub struct SphericalConstraint {
    pub node_a: IkNodeID,
    pub node_b: IkNodeID,
    pub point_in_a: Vec3A,
    pub point_in_b: Vec3A,
}

pub struct DistanceConstraint {
    pub node_a: IkNodeID,
    pub node_b: IkNodeID,
    pub point_in_a: Vec3A,
    pub point_in_b: Vec3A,
    pub distance: f32,
}

// The angular part of a cardan (universal) joint.
// Should be combined with a spherical constraint for a regular cardan joint.
pub struct AngularCardanConstraint {
    pub node_a: IkNodeID,
    pub node_b: IkNodeID,
    pub axis_in_a: Vec3A,
    pub axis_in_b: Vec3A,
}

pub struct CompliantSphericalConstraint {
    pub node_a: IkNodeID,
    pub node_b: IkNodeID,
    pub point_in_a: Vec3A,
    pub point_in_b: Vec3A,
    pub compliance: f32,
}

pub struct CompliantFixedAngleConstraint {
    pub node_a: IkNodeID,
    pub node_b: IkNodeID,
    pub b_in_a: Quat,
    pub compliance: f32,
}

pub struct CompliantHingeAngleConstraint {
    pub node_a: IkNodeID,
    pub node_b: IkNodeID,
    pub axis_in_a: Vec3A,
    pub axis_in_b: Vec3A,
    pub compliance: f32,
}
