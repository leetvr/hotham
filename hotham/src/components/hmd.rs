/// A marker component used to indicate the player's headset, or Head Mounted Display in the game simulation.
///
/// The entity marked with this component will have its [`super::LocalTransform`] is updated each frame by the
/// engine with the pose of the player's headset in the real world (ie. stage space). Since this entity is
/// parented to the [`super::Stage`] entity, querying for the HMD's [`super::GlobalTransform`] will then give
/// the pose of the HMD in the virtual world (ie. global space).
///
/// This is very important when incorporating the user's position in the real world into the game simulation,
/// ie. player controllers. Future versions of Hotham may add more functionality to make this even easier.
pub struct HMD {}
