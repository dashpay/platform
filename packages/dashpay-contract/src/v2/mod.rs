use crate::error::Error;
use serde_json::Value;

// Document-type name and property constants live in `crate::v1::document_types`;
// v2 does not change any names v1 defined, it only adds the optional
// `corePaymentAddress` / `platformPaymentAddress` properties to `profile`.

pub fn load_documents_schemas() -> Result<Value, Error> {
    serde_json::from_str(include_str!("../../schema/v2/dashpay.schema.json"))
        .map_err(Error::InvalidSchemaJson)
}
