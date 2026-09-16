use crate::Error;
use serde_json::Value;

/// The document type names and property names are unchanged from v1 — see
/// [`crate::v1::document_types`]. v2 only subscribes the `domain` document
/// type to the document history system contract via the
/// `keepsTransferHistory`, `keepsPurchaseHistory` and `keepsPricingHistory`
/// configuration flags.
pub fn load_documents_schemas() -> Result<Value, Error> {
    serde_json::from_str(include_str!("../../schema/v2/dpns-contract-documents.json"))
        .map_err(Error::InvalidSchemaJson)
}
