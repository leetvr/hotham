/// Buffers are used to transfer data to the GPU
pub mod buffer;

/// Buffers are used to transfer data to the GPU
pub mod gambier_buffer;

/// Functionality for interacting with GPU memory
pub mod memory;

/// The virtual camera
pub mod camera;

/// A wrapper around the frame-dependent resources
pub mod frame;

/// A wrapper around an image
pub mod image;

/// Shared data for a scene
pub mod scene_data;

/// Helper wrapper for interacting with the swapchain
pub mod swapchain;

/// Functionality for adding textures (images) to meshes
pub mod texture;

/// Vertex representation
pub mod vertex;

/// A wrapper for all Descriptor related functionality
pub(crate) mod descriptors;

/// Container for all the resources used to render objects
pub mod resources;
