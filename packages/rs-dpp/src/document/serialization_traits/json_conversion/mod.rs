mod v0;

pub use v0::*;

use crate::document::Document;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;
use serde_json::Value as JsonValue;

impl DocumentJsonMethodsV0<'_> for Document {
    /// Validating-JSON shape: bs58 string identifiers + binary fields as
    /// JSON arrays of u8. Used by JSON Schema validators that don't
    /// accept base64 string encodings.
    fn to_json_with_identifiers_using_bytes(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<JsonValue, ProtocolError> {
        match self {
            Document::V0(v0) => v0.to_json_with_identifiers_using_bytes(platform_version),
        }
    }
}
