#[derive(Debug, Clone, PartialEq, Copy)]
pub enum Colour {
    Red,
    Blue,
}

#[derive(Debug, Clone)]
pub struct Cube {
    pub colour: Colour,
}
