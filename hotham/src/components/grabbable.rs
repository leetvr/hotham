use hecs::Entity;

#[derive(Debug, Clone, Copy)]
pub struct Grabbable;

#[derive(Debug, Clone, Copy)]
pub struct Grabbed {
    pub hand: Entity,
}

#[derive(Debug, Clone, Copy)]
pub struct Released;
