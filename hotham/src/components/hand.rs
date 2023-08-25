use glam::Affine3A;
use hecs::Entity;

/// A component that represents the "side" or "handedness" that an entity is on
/// Used by components such as `Hand` and `Pointer` to identify which controller they should map to
#[derive(Debug, PartialEq, Clone, Copy, Eq, PartialOrd, Ord)]
pub enum Handedness {
    /// Left hand side
    Left,
    /// Right hand side
    Right,
}

#[derive(Clone)]
pub struct GrabbedEntity {
    pub entity: Entity,
    pub grip_from_local: Affine3A,
}

/// A component that's added to an entity to represent a "hand" presence.
/// Used to give the player a feeling of immersion by allowing them to grab objects in the world
/// Requires `hands_system`
#[derive(Clone)]
pub struct Hand {
    /// How much has this hand been gripped?
    pub grip_value: f32,
    /// Did the grip button go from not pressed to pressed this frame?
    pub grip_button_just_pressed: bool,
    /// Which side is this hand on?
    pub handedness: Handedness,
    /// Have we grabbed something?
    pub grabbed_entity: Option<GrabbedEntity>,
}

impl Hand {
    /// Shortcut helper to create a Left hand
    pub fn left() -> Hand {
        Hand {
            grip_value: 0.0,
            grip_button_just_pressed: false,
            handedness: Handedness::Left,
            grabbed_entity: None,
        }
    }

    /// Shortcut helper to create a right hand
    pub fn right() -> Hand {
        Hand {
            grip_value: 0.0,
            grip_button_just_pressed: false,
            handedness: Handedness::Right,
            grabbed_entity: None,
        }
    }
}
