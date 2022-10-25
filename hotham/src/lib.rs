#![deny(missing_docs)]
// TODO Safety doc would be nice
#![allow(clippy::missing_safety_doc)]

//! G'day, and welcome to Hotham! 👋
//!
//! Hotham is an attempt to create a lightweight, high performance game engine for mobile VR headsets. It's primarily aimed at small (1-5 person) teams of mostly technical folk who are looking to create games for devices like the Oculus Quest, but find existing tools cumbersome to work with. You can learn more about the project [in the FAQ](https://github.com/leetvr/hotham/wiki/FAQ).
//!
//! # Getting started
//! Hotham is a complex project with many moving parts! Have no fear - we've written an easy to follow [Getting Started guide](https://github.com/leetvr/hotham/wiki/Getting-started) that will have you running our example application in no time. Head on over to [getting started](https://github.com/leetvr/hotham/wiki/Getting-started) to.. get.. started.
//!
//! # Sponsoring
//! Hotham's development is only possible thanks to the support of the community. It's currently being developed on full time by [@kanerogers](https://github.com/kanerogers) If you'd like to help make VR development in Rust possible, please [consider becoming a donor](https://github.com/sponsors/leetvr). 💗

pub use ash::vk;
pub use openxr as xr;

pub use engine::{Engine, EngineBuilder, TickData};
pub use glam;
pub use hecs;
pub use hotham_error::HothamError;

/// Components are data that are used to update the simulation and interact with the external world
pub mod components;
mod engine;

/// A tool to import models from glTF files into Hotham
pub mod asset_importer;
/// Contexts are wrappers around some external state that the engine will interact with
pub mod contexts;
mod hotham_error;
/// Systems are functions called each frame to update either the external state or the current simulation
pub mod systems;

/// Kitchen sink utility functions
pub mod util;

/// Functionality used by the rendering engine
pub mod rendering;
mod workers;

/// Hotham result type
pub type HothamResult<T> = std::result::Result<T, HothamError>;

/// Format used for color textures
pub const COLOR_FORMAT: vk::Format = vk::Format::R8G8B8A8_SRGB;
/// Format used for depth textures
pub const DEPTH_FORMAT: vk::Format = vk::Format::D32_SFLOAT;

/// Number of views
pub const VIEW_COUNT: u32 = 2;

/// Swapchain length
pub const SWAPCHAIN_LENGTH: usize = 3;

/// OpenXR view type
pub const VIEW_TYPE: xr::ViewConfigurationType = xr::ViewConfigurationType::PRIMARY_STEREO;

/// OpenXR blend mode
pub const BLEND_MODE: xr::EnvironmentBlendMode = xr::EnvironmentBlendMode::OPAQUE;
