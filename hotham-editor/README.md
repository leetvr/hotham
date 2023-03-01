# Hotham Editor
> 🚧 **UNDER CONSTRUCTION** 🚧
>
> **WARNING**: Even more so than the rest of Hotham, this crate is under *heavy* construction. While it is technically usable in its current state its design may change: at any time. You have been warned!

## Future plans
The ideal state for the Hotham editor is that it will make it easier to build VR games. That's a pretty ambitious goal, but that is, after all, the whole *point* of Hotham.

Game Editors are notoriously tricky pieces of technology to define, but here are a couple of things we'd like to be able to do:

1. Define the *initial state* for some part of a game (eg. a "scene" or a "level")
1. Inspect the *current state* of the game to debug it

That shouldn't be too hard, right?

It's also worth noting that the editor will *completely replace* Hotham simulator.

## Current state
Currently, it's possible to do the following:

- Run `simple-scene-example` with the `editor` feature
- Manipulate the transform of an entity
