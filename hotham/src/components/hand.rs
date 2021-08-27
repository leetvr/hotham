use legion::Entity;

#[derive(Debug, PartialEq, Clone, Copy, Eq, PartialOrd, Ord)]
pub enum Handedness {
    Left,
    Right,
}

#[derive(Clone)]
pub struct Hand {
    pub grip_value: f32,
    pub handedness: Handedness,
    pub grabbed_entity: Option<Entity>,
}

impl Hand {
    pub fn left() -> Hand {
        Hand {
            grip_value: 0.0,
            handedness: Handedness::Left,
            grabbed_entity: None,
        }
    }

    pub fn right() -> Hand {
        Hand {
            grip_value: 0.0,
            handedness: Handedness::Right,
            grabbed_entity: None,
        }
    }
}
