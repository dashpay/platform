use thiserror::Error;

#[derive(Error, Debug, bincode::Encode, bincode::Decode)]
pub enum PlatformVersionError {
    #[error("unknown version error {0}")]
    UnknownVersionError(String),
}
