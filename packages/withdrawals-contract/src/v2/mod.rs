use crate::error::Error;
use serde_json::Value;

// Document-type name and property constants live in `crate::v1::document_types`;
// v2 does not change any names v1 defined, it only admits the terminal
// `FAILED` (5) value of the `status` property.

pub(super) fn load_documents_schemas() -> Result<Value, Error> {
    serde_json::from_str(include_str!("../../schema/v2/withdrawals-documents.json"))
        .map_err(Error::InvalidSchemaJson)
}
