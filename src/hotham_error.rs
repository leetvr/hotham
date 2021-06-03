use ash::{vk, InstanceError, LoadingError};
use openxr::sys::Result as OpenXRResult;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HothamError {
    #[error("Vulkan instance error:")]
    VulkanInstanceError {
        #[from]
        source: InstanceError,
    },
    #[error("Vulkan error:")]
    VulkanError {
        #[from]
        source: vk::Result,
    },
    #[error("Error loading Vulkan:")]
    VulkanLoadingError {
        #[from]
        source: LoadingError,
    },
    #[error("OpenXR error:")]
    OpenXRError {
        #[from]
        source: OpenXRResult,
    },
    #[error("The list was empty")]
    EmptyListError,
    #[error("The version of Vulkan or OpenXR is not supported")]
    UnsupportedVersionError,
}
