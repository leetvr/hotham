use ash::vk::Result as VulkanResult;
use openxr::sys::Result as OpenXRResult;
use thiserror::Error;

/// Hotham Error type
#[derive(Error, Debug)]
pub enum HothamError {
    /// OpenXR error
    #[error("There was a problem with an OpenXR operation")]
    OpenXRError(#[from] OpenXRResult),
    /// Vulkan error
    #[error("There was a problem with a Vulkan operation")]
    VulkanError(#[from] VulkanResult),
    /// An empty list // TODO: useless
    #[error("The list was empty")]
    EmptyListError,
    /// Unsupported version
    #[error("the version of vulkan or openxr is not supported")]
    UnsupportedVersionError,
    /// Invalid format
    #[error("The format provided - {format:?} - is not supported for this operation")]
    InvalidFormatError {
        /// The format that was invalid
        format: String,
    },
    /// Engine shutting down
    #[error("The engine is shutting down")]
    ShuttingDown,
    /// IO error
    #[error(transparent)]
    IO(#[from] std::io::Error),
    /// Not rendering yet
    #[error("this session is not rendering yet")]
    NotRendering,
    /// Some other error
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
