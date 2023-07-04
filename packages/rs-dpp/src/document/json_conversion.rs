use crate::document::v0::json_conversion::DocumentV0JsonMethods;
use crate::document::{Document, DocumentV0, DocumentV0Methods};
use crate::ProtocolError;
use platform_value::Identifier;
use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::convert::TryInto;

impl DocumentV0JsonMethods for Document {
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
    fn from_json_value<S>(mut document_value: JsonValue) -> Result<Self, ProtocolError>
    where
        for<'de> S: Deserialize<'de> + TryInto<Identifier, Error = ProtocolError>,
    {
        Ok(Document::V0(DocumentV0::from_json_value::<S>(
            document_value,
        )?))
    }
}
