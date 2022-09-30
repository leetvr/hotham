/// A component to assist with artificial locomotion.
///
/// On startup, [`crate::Engine`] will create an entity with the [`Stage`] component, and an entity with the
/// [`super::HMD`] component (representing the user's headset), parented to the stage entity.
///
/// This makes moving the player around in the virtual world *relatively* straightforward, as all is required is
/// to move the [`Stage`]. The engine will ensure that the player's position in the game simulation AND the
/// virtual cameras are correctly updated to account for this.
///
/// In short, the final position of the player in the game simulation (ie. global space) is:
///
/// `stage.position * hmd.position`
///
/// *You* are responsible for controlling the [`Stage`], and the *engine* will update the [`super::HMD`].
///
/// For more information on how this works, check out [`super::HMD`] and [`crate::contexts::InputContext`].
#[derive(Debug)]
pub struct Stage;

use glam::Affine3A;
use hecs::With;

use crate::{components::GlobalTransform, hecs::World};

/// Get the transform of the stage in global space.
pub fn get_global_from_stage(world: &World) -> Affine3A {
    // Get the stage transform
    world
        .query::<With<Stage, &GlobalTransform>>()
        .into_iter()
        .next()
        .map(|(_, global_transform)| global_transform.0)
        .unwrap_or(Affine3A::IDENTITY)
}
