/// Component that adds some information about the entity
/// Useful for debugging - added by default by `gltf_loader`
#[derive(Debug, Clone, Eq, PartialEq, Hash, Default)]
pub struct Info {
    /// A helpful name
    pub name: String,
    /// Node ID from the original glTF file
    pub node_id: usize,
}
