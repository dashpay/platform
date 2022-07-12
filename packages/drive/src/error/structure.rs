#[derive(Debug, thiserror::Error)]
pub enum StructureError {
    #[error("invalid protocol version error: {0}")]
    InvalidProtocolVersion(&'static str),

    #[error("invalid cbor error: {0}")]
    InvalidCBOR(&'static str),

    #[error("key wrong type error: {0}")]
    KeyWrongType(&'static str),

    #[error("key out of bounds error: {0}")]
    KeyWrongBounds(&'static str),

    #[error("value wrong type error: {0}")]
    ValueWrongType(&'static str),
}
