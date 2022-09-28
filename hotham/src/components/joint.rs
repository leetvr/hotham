use glam::Affine3A;
use hecs::Entity;

/// A component that adds a "skinned joint" to an entity.
/// For more detail, check out the [glTF spec](https://www.khronos.org/registry/glTF/specs/2.0/glTF-2.0.html#skins-overview)
/// Automatically added by `gltf_loader` for nodes that contain skin data
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Joint {
    /// Pointer to the root of the skeleton
    pub skeleton_root: Entity,
    /// Inverse bind matrix used to apply the skin in the vertex shader
    pub inverse_bind_matrix: Affine3A,
}
