use thiserror::Error;

#[derive(Error, Debug)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub enum PlatformVersionError {
    #[error("unknown version error {0}")]
    UnknownVersionError(String),
}
