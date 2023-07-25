use thiserror::Error;

#[derive(Error, Debug)]
pub enum PlatformVersionError {
    #[error("unknown version error {0}")]
    UnknownVersionError(String),
}
