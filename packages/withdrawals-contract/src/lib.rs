use lazy_static::lazy_static;
use serde_json::Value;

lazy_static! {
    pub static ref DOCUMENT_SCHEMAS: Value =
        serde_json::from_str(include_str!("../schema/withdrawals-documents.json")).unwrap();
}
