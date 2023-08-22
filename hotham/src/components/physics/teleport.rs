/// A "one-shot" tag component that indicates to the physics system that you would like
/// this entity to be teleported - ie. moved in a non-physical way. The mechanism for this is simple:
///
/// 1. Insert this component to the entity you'd like to teleport
/// 2. Set the entity's GlobalTransform to where you'd like the entity teleported
///
/// In the next tick, this component will be removed.
pub struct Teleport {}
