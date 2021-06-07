use thiserror::Error;

#[derive(Error, Debug)]
pub enum HothamError {
    #[error(transparent)]
    Other(#[from] anyhow::Error),
    #[error("The list was empty")]
    EmptyListError,
    #[error("The version of Vulkan or OpenXR is not supported")]
    UnsupportedVersionError,
    #[error("The format provided is not supported for this operation")]
    InvalidFormatError,
}
