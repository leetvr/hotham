use super::Transform;
use legion::Entity;

#[derive(Debug, Clone, PartialEq)]
pub struct AnimationTarget {
    pub controller: Entity,
    pub animations: Vec<Vec<Transform>>,
}
