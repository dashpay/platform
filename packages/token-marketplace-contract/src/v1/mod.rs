use crate::Error;
use serde_json::Value;

pub fn load_documents_schemas() -> Result<Value, Error> {
    serde_json::from_str(include_str!("../../schema/v1/documents.json"))
        .map_err(Error::InvalidSchemaJson)
}
