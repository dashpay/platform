use crate::Error;
use serde_json::Value;

pub mod document_types {
    pub mod transfer {
        pub const NAME: &str = "transfer";

        pub mod properties {
            pub const DATA_CONTRACT_ID: &str = "dataContractId";
            pub const DOCUMENT_TYPE_NAME: &str = "documentTypeName";
            pub const DOCUMENT_ID: &str = "documentId";
            pub const TO_IDENTITY_ID: &str = "toIdentityId";
        }
    }

    pub mod purchase {
        pub const NAME: &str = "purchase";

        pub mod properties {
            pub const DATA_CONTRACT_ID: &str = "dataContractId";
            pub const DOCUMENT_TYPE_NAME: &str = "documentTypeName";
            pub const DOCUMENT_ID: &str = "documentId";
            pub const SELLER_ID: &str = "sellerId";
            pub const PRICE: &str = "price";
        }
    }

    pub mod price_update {
        pub const NAME: &str = "priceUpdate";

        pub mod properties {
            pub const DATA_CONTRACT_ID: &str = "dataContractId";
            pub const DOCUMENT_TYPE_NAME: &str = "documentTypeName";
            pub const DOCUMENT_ID: &str = "documentId";
            pub const PRICE: &str = "price";
        }
    }
}

pub fn load_documents_schemas() -> Result<Value, Error> {
    serde_json::from_str(include_str!(
        "../../schema/v1/document-history-contract-documents.json"
    ))
    .map_err(Error::InvalidSchemaJson)
}
