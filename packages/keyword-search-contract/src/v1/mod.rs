use crate::Error;
use serde_json::Value;

pub mod document_types {
    pub mod contract_keywords {
        pub const NAME: &str = "contractKeywords";

        pub mod properties {
            pub const KEY_INDEX: &str = "byKeyword";
        }
    }

    pub mod short_description {
        pub const NAME: &str = "shortDescription";

        pub mod properties {
            pub const KEY_INDEX: &str = "byContractId";
        }
    }

    pub mod full_description {
        pub const NAME: &str = "fullDescription";

        pub mod properties {
            pub const KEY_INDEX: &str = "byContractId";
        }
    }
}

pub fn load_documents_schemas() -> Result<Value, Error> {
    serde_json::from_str(include_str!(
        "../../schema/v1/keyword-search-contract-documents.json"
    ))
    .map_err(Error::InvalidSchemaJson)
}
