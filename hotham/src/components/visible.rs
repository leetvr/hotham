/// The Visibility component determines whether a given entity is shown or hidden within the world.
///
/// During each tick of the Hotham engine, entities can have Visibility assigned or removed.
///
/// Basic usage:
/// ```
/// world.insert_one(entity, Visible {})
/// world.remove_one::<Visible>(entity)
/// ```

#[derive(Debug, Clone, Copy)]
pub struct Visible {}
