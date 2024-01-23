use crate::error::Error;
use serde_json::Value;

pub mod document_types {
    pub mod contact_request {
        pub const NAME: &str = "contactRequest";

        pub mod properties {
            pub const TO_USER_ID: &str = "toUserId";
            pub const CORE_HEIGHT_CREATED_AT: &str = "coreHeightCreatedAt";
            pub const CORE_CHAIN_LOCKED_HEIGHT: &str = "coreChainLockedHeight";
        }
    }
}

pub fn load_documents_schemas() -> Result<Value, Error> {
    serde_json::from_str(include_str!("../../schema/v1/dashpay.schema.json"))
        .map_err(Error::InvalidSchemaJson)
}
