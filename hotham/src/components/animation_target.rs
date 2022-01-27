use super::Transform;
use hecs::Entity;

#[derive(Debug, Clone, PartialEq)]
pub struct AnimationTarget {
    pub controller: Entity,
    pub animations: Vec<Vec<Transform>>,
}
