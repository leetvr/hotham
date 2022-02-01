pub mod game;
pub mod sabers;
use hotham::{
    components::{Collider, RigidBody, Visible},
    hecs::{PreparedQuery, With, Without},
};
pub use sabers::sabers_system;

use crate::components::{Colour, Cube, Saber};

#[derive(Default)]
pub struct BeatSaberQueries<'a> {
    pub sabers_query: PreparedQuery<With<Saber, (&'a Colour, &'a RigidBody)>>,
    pub live_cubes_query:
        PreparedQuery<With<Visible, With<Cube, (&'a Colour, &'a RigidBody, &'a Collider)>>>,
    pub dead_cubes_query: PreparedQuery<Without<Visible, With<Cube, &'a Colour>>>,
}
