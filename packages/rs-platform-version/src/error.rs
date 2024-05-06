use thiserror::Error;

#[derive(Error, Debug)]
#[ferment_macro::export]
pub enum PlatformVersionError {
    #[error("unknown version error {0}")]
    UnknownVersionError(String),
}
