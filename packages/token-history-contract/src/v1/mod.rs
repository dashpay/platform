use crate::Error;
use serde_json::Value;

pub mod document_types {
    pub mod tx_metadata {
        pub const NAME: &str = "tx_metadata";

        pub mod properties {
            pub const KEY_INDEX: &str = "keyIndex";
            pub const ENCRYPTION_KEY_INDEX: &str = "encryptionKeyIndex";
            pub const ENCRYPTED_METADATA: &str = "encryptedMetadata";
        }
    }
}

pub fn load_documents_schemas() -> Result<Value, Error> {
    serde_json::from_str(include_str!(
        "../../schema/v1/token-history-contract-documents.json"
    ))
    .map_err(Error::InvalidSchemaJson)
}
