// Empty struct, used as a "tag" to indicate this mesh has a skin.
#[derive(Debug, Clone, PartialEq)]
pub struct Skin {
    pub joint_ids: Vec<usize>,
}
