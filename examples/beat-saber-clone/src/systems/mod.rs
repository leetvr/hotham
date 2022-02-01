pub mod game;
pub mod sabers;
use hotham::{
    components::RigidBody,
    hecs::{PreparedQuery, With},
};
pub use sabers::sabers_system;

use crate::components::{Colour, Cube, Saber};

#[derive(Default)]
pub struct BeatSaberQueries<'a> {
    pub sabers_query: PreparedQuery<With<Saber, (&'a Colour, &'a RigidBody)>>,
    pub cubes_query: PreparedQuery<&'a Cube>,
}
