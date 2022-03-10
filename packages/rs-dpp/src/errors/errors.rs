use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("Identifier Error: {0}")]
    IdentifierError(String),
    #[error("String Decode Error {0}")]
    StringDecodeError(String),
    #[error("Public key data is not set")]
    EmptyPublicKeyDataError,
    #[error("Payload reached a {0}Kb limit")]
    MaxEncodedBytesReachedError(usize),
    #[error("Encoding Error - {0}")]
    EncodingError(String),
    #[error(transparent)]
    Error(#[from] anyhow::Error),
}
