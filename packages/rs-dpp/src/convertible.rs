use platform_value::Value;
use serde_json::Value as JsonValue;

use crate::ProtocolError;

pub trait Convertible {
    /// Returns the [`platform_value::Value`] instance on an object
    #[cfg(feature = "platform-value")]
    fn to_object(&self) -> Result<Value, ProtocolError>;
    #[cfg(feature = "platform-value")]
    fn to_cleaned_object(&self) -> Result<Value, ProtocolError>;
    /// Returns the [`platform_value::Value`] instance on an object
    #[cfg(feature = "platform-value")]
    fn into_object(self) -> Result<Value, ProtocolError>;
    /// Returns the [`serde_json::Value`] instance that preserves the `Vec<u8>` representation
    /// for Identifiers and binary data
    #[cfg(feature = "json-object")]
    fn to_json_object(&self) -> Result<JsonValue, ProtocolError>;
    /// Returns the [`serde_json::Value`] instance that encodes:
    ///  - Identifiers  - with base58
    ///  - Binary data  - with base64
    #[cfg(feature = "json-object")]
    fn to_json(&self) -> Result<JsonValue, ProtocolError>;
    /// Returns the cbor-encoded bytes representation of the object. The data is prefixed by 4 bytes containing
    /// the Protocol Version
    #[cfg(feature = "cbor")]
    fn to_cbor_buffer(&self) -> Result<Vec<u8>, ProtocolError>;
}
