use crate::Error;
use serde_json::Value;

pub mod document_types {
    pub mod contract {
        pub const NAME: &str = "contract";

        pub mod properties {
            pub const KEY_INDEX: &str = "byKeyword";
        }
    }

    pub mod token {
        pub const NAME: &str = "token";

        pub mod properties {
            pub const KEY_INDEX: &str = "byKeyword";
        }
    }
}

pub fn load_documents_schemas() -> Result<Value, Error> {
    serde_json::from_str(include_str!(
        "../../schema/v1/search-contract-documents.json"
    ))
    .map_err(Error::InvalidSchemaJson)
}
