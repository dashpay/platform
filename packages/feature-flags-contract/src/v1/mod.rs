use crate::error::Error;
use serde_json::Value;

pub mod document_types {
    pub mod update_consensus_params {
        pub const NAME: &str = "updateConsensusParams";

        pub mod properties {
            pub const PROPERTY_BLOCK_HEIGHT: &str = "height";
            pub const PROPERTY_ENABLE_AT_HEIGHT: &str = "enableAtHeight";
        }
    }
}

pub fn load_documents_schemas() -> Result<Value, Error> {
    serde_json::from_str(include_str!("../../schema/v1/feature-flags-documents.json"))
        .map_err(Error::InvalidSchemaJson)
}
