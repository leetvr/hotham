pub mod game;
pub mod sabers;
use hotham::{
    components::{Collider, RigidBody, Visible},
    hecs::{PreparedQuery, With},
};
pub use sabers::sabers_system;

use crate::components::{Color, Cube, Saber};

#[derive(Default)]
pub struct CrabSaberQueries<'a> {
    pub sabers_query: PreparedQuery<With<Saber, (&'a Color, &'a RigidBody)>>,
    #[cfg_attr(feature = "cargo-clippy", allow(clippy::type_complexity))]
    pub live_cubes_query:
        PreparedQuery<With<Visible, With<Cube, (&'a Color, &'a RigidBody, &'a Collider)>>>,
}
