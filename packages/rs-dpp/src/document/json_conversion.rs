use crate::document::serialization_traits::DocumentJsonMethodsV0;
use crate::document::{Document, DocumentV0};
use crate::ProtocolError;
use platform_value::Identifier;
use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::convert::TryInto;
use platform_version::version::PlatformVersion;

impl<'a> DocumentJsonMethodsV0<'a> for Document {
    /// Convert the document to JSON with identifiers using bytes.
    fn to_json_with_identifiers_using_bytes(&self) -> Result<JsonValue, ProtocolError> {
        match self {
            Document::V0(v0) => v0.to_json_with_identifiers_using_bytes(),
        }
    }

    /// Convert the document to a JSON value.
    fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        match self {
            Document::V0(v0) => v0.to_json(),
        }
    }

    /// Create a document from a JSON value.
    fn from_json_value<S>(mut document_value: JsonValue, platform_version: &PlatformVersion) -> Result<Self, ProtocolError>
    where
        for<'de> S: Deserialize<'de> + TryInto<Identifier, Error = ProtocolError>,
    {
        match platform_version.dpp.document_versions.document_structure_version {
            0 => {
                Ok(Document::V0(DocumentV0::from_json_value::<S>(
                    document_value,
                    platform_version,
                )?))
            }
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "Document::from_json_value".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }

    }
}
