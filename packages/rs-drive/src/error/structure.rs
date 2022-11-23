/// Structure errors
#[derive(Debug, thiserror::Error)]
pub enum StructureError {
    /// Invalid protocol version error
    #[error("invalid protocol version error: {0}")]
    InvalidProtocolVersion(&'static str),
    /// Invalid CBOR error
    #[error("invalid cbor error: {0}")]
    InvalidCBOR(&'static str),
    /// Key wrong type error
    #[error("key wrong type error: {0}")]
    KeyWrongType(&'static str),
    /// Key wrong bounds error
    #[error("key out of bounds error: {0}")]
    KeyWrongBounds(&'static str),
    /// Value wrong type error
    #[error("value wrong type error: {0}")]
    ValueWrongType(&'static str),
}
