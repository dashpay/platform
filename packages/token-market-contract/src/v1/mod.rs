use crate::error::Error;
use serde_json::Value;

pub mod document_types {
    /// Tokens a holder offers for sale at a price, held in escrow until filled or cancelled.
    pub mod ask {
        pub const NAME: &str = "ask";

        pub mod properties {
            pub const TOKEN_ID: &str = "tokenId";
            pub const PRICE: &str = "price";
            pub const AMOUNT: &str = "amount";
            pub const REMAINING: &str = "remaining";
            pub const MIN_FILL: &str = "minFill";
            pub const EXPIRES_AT: &str = "expiresAt";
        }

        pub mod indexes {
            pub const BOOK: &str = "book";
            pub const DEPTH: &str = "depth";
            pub const BY_OWNER: &str = "byOwner";
        }
    }

    /// An offer to buy a token at a price, its credits held in escrow until filled or
    /// cancelled.
    pub mod bid {
        pub const NAME: &str = "bid";

        pub mod properties {
            pub const TOKEN_ID: &str = "tokenId";
            pub const PRICE: &str = "price";
            pub const AMOUNT: &str = "amount";
            pub const REMAINING: &str = "remaining";
            pub const MIN_FILL: &str = "minFill";
            pub const EXPIRES_AT: &str = "expiresAt";
            pub const ESCROW: &str = "escrow";
        }

        pub mod indexes {
            pub const BOOK: &str = "book";
            pub const DEPTH: &str = "depth";
            pub const BY_OWNER: &str = "byOwner";
        }
    }
}

pub fn load_documents_schemas() -> Result<Value, Error> {
    serde_json::from_str(include_str!(
        "../../schema/v1/token-market-contract-documents.json"
    ))
    .map_err(Error::InvalidSchemaJson)
}
