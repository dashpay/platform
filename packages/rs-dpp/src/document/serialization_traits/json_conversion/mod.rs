mod v0;

pub use v0::*;

use crate::document::{Document, DocumentV0};
use crate::ProtocolError;
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::convert::TryInto;

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

    /// Legacy-shape ingest: generic over identifier deserialization type
    /// (`String` for bs58 / `Vec<u8>` for raw), manually parses each
    /// system field, and accepts JSON without a `$formatVersion` tag.
    fn from_json_value<S, E>(
        document_value: JsonValue,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError>
    where
        for<'de> S: Deserialize<'de> + TryInto<Identifier, Error = E>,
        E: Into<ProtocolError>,
    {
        match platform_version
            .dpp
            .document_versions
            .document_structure_version
        {
            0 => Ok(Document::V0(DocumentV0::from_json_value::<S, E>(
                document_value,
                platform_version,
            )?)),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Document::from_json_value".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
