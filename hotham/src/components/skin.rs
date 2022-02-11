/// Component added to an entity to point to the joints in the node
/// Automatically added by `gltf_loader`
#[derive(Debug, Clone, PartialEq)]
pub struct Skin {
    /// List of joints, represented by their node ID
    pub joint_ids: Vec<usize>,
}
