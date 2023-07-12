use std::collections::BTreeMap;
use platform_value::Value;
use crate::document::{Document, DocumentV0};
use crate::document::property_names::FEATURE_VERSION;
use crate::document::serialization_traits::DocumentPlatformValueMethodsV0;
use crate::ProtocolError;
use crate::version::FeatureVersion;

impl DocumentPlatformValueMethodsV0 for Document {
    /// Convert the document to a map value.
    fn to_map_value(&self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        match self {
            Document::V0(v0) => v0.to_map_value(),
        }
    }

    /// Convert the document to a map value consuming the document.
    fn into_map_value(self) -> Result<BTreeMap<String, Value>, ProtocolError> {
        match self {
            Document::V0(v0) => v0.into_map_value(),
        }
    }

    /// Convert the document to a value consuming the document.
    fn into_value(self) -> Result<Value, ProtocolError> {
        match self {
            Document::V0(v0) => v0.into_value(),
        }
    }

    /// Convert the document to an object.
    fn to_object(&self) -> Result<Value, ProtocolError> {
        match self {
            Document::V0(v0) => v0.to_object(),
        }
    }

    /// Create a document from a platform value.
    fn from_platform_value(document_value: Value) -> Result<Self, ProtocolError> {
        let version: FeatureVersion = document_value.get_integer(FEATURE_VERSION)?;
        match version {
            0 => Ok(Document::V0(DocumentV0::from_platform_value(
                document_value,
            )?)),
            version => Err(ProtocolError::UnknownVersionError(format!(
                "version {version} not known for document for call from_platform_value"
            ))),
        }
    }
}