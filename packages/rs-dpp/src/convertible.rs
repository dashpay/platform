use crate::ProtocolError;
use serde_json::Value as JsonValue;

pub trait Convertible {
    /// Returns the [`serde_json::Value`] instance that preserves the `Vec<u8>` representation
    /// for Identifiers and binary data
    fn to_object(&self) -> Result<JsonValue, ProtocolError>;
    /// Returns the [`serde_json::Value`] instance that encodes:
    ///  - Identifiers  - with base58
    ///  - Binary data  - with base64
    fn to_json(&self) -> Result<JsonValue, ProtocolError>;
    // Returns the cibor-encoded bytes representation of the object. The data is prefixed by 4 bytes containing
    // the Protocol Version
    fn to_buffer(&self) -> Result<Vec<u8>, ProtocolError>;
}
