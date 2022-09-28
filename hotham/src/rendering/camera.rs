use glam::{Affine3A, Mat4, Vec4};
use openxr as xr;

use crate::util::affine_from_posef;

#[derive(Debug, Clone)]
/// The Camera, or View, in a scene.
pub struct Camera {
    /// The camera's pose in globally oriented stage space
    pub gos_from_view: Affine3A,
    /// The view matrix
    pub view_from_gos: Mat4,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            gos_from_view: Affine3A::IDENTITY,
            view_from_gos: Mat4::IDENTITY,
        }
    }
}

impl Camera {
    /// Update the camera's position from an OpenXR view
    pub fn update(&mut self, view: &xr::View, gos_from_stage: &Affine3A) -> Mat4 {
        // Convert values from OpenXR format and use globally oriented stage space instead of stage space
        let stage_from_view = affine_from_posef(view.pose);
        self.gos_from_view = *gos_from_stage * stage_from_view;

        self.view_from_gos = self.build_matrix();
        self.view_from_gos
    }

    /// Get the camera's position in homogenous coordinates
    pub fn position_in_gos(&self) -> Vec4 {
        let p = self.gos_from_view.translation;
        [p[0], p[1], p[2], 1.].into()
    }

    /// Build the camera's view matrix
    pub fn build_matrix(&self) -> Mat4 {
        self.gos_from_view.inverse().into()
    }
}

#[derive(Debug, Copy, Clone)]
/// A frustrum for the virtual camera.
pub struct Frustum {
    /// The left angle
    pub left: f32,
    /// The right angle
    pub right: f32,
    /// The top angle
    pub up: f32,
    /// The bottom angle
    pub down: f32,
}

impl Frustum {
    #[rustfmt::skip]
    /// Compute right-handed y-up inverse Z perspective projection matrix
    pub fn projection(&self, znear: f32) -> Mat4 {
        // Based on http://dev.theomader.com/depth-precision/ + OpenVR docs
        let left = self.left.tan();
        let right = self.right.tan();
        let down = self.down.tan();
        let up = self.up.tan();
        let idx = 1.0 / (right - left);
        let idy = 1.0 / (down - up);
        let sx = right + left;
        let sy = down + up;

        // TODO: This was originally written using nalgebra's row-order format, so we just
        // transpose the resulting matrix. We should probably just.. you know, rewrite this.
        Mat4::from_cols_array(&[
            2.0 * idx, 0.0, sx * idx, 0.0,
            0.0, 2.0 * idy, sy * idy, 0.0,
            0.0,       0.0,      0.0, znear,
            0.0,       0.0,     -1.0, 0.0]).transpose()
    }
}

impl From<xr::Fovf> for Frustum {
    fn from(x: xr::Fovf) -> Self {
        Self {
            left: x.angle_left,
            right: x.angle_right,
            up: x.angle_up,
            down: x.angle_down,
        }
    }
}

/// Normals of the clipping planes are pointing towards the inside of the frustum.
/// We are only using four planes per camera. The near and far planes are not used.
/// This link points to a paper describing the math behind these expressions:
/// https://www.gamedevs.org/uploads/fast-extraction-viewing-frustum-planes-from-world-view-projection-matrix.pdf
pub(crate) fn extract_planes_from_frustum(frustum: &Mat4) -> Mat4 {
    // Glam doesn't support creating matrices from rows, so we'll just use columns and transpose.
    Mat4::from_cols(
        normalize_plane(frustum.row(3) + frustum.row(0)),
        normalize_plane(frustum.row(3) - frustum.row(0)),
        normalize_plane(frustum.row(3) + frustum.row(1)),
        normalize_plane(frustum.row(3) - frustum.row(1)),
    )
    .transpose()
}

pub(crate) fn normalize_plane(p: Vec4) -> Vec4 {
    p / p.truncate().length()
}
