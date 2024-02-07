use crate::error::Error;
use serde_json::Value;

pub mod document_types {
    pub mod reward_share {
        pub const NAME: &str = "rewardShare";

        pub mod properties {
            pub const PAY_TO_ID: &str = "payToId";
            pub const PERCENTAGE: &str = "percentage";
        }
    }
}

pub(super) fn load_documents_schemas() -> Result<Value, Error> {
    serde_json::from_str(include_str!(
        "../../schema/v1/masternode-reward-shares-documents.json"
    ))
    .map_err(Error::InvalidSchemaJson)
}
