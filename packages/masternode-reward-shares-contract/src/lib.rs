use platform_value::{Identifier, IdentifierBytes32};
use serde_json::Error;
use serde_json::Value;

pub const ID_BYTES: [u8; 32] = [
    12, 172, 226, 5, 36, 102, 147, 167, 200, 21, 101, 35, 98, 13, 170, 147, 125, 47, 34, 71, 147,
    68, 99, 238, 176, 31, 247, 33, 149, 144, 149, 140,
];

pub const OWNER_ID_BYTES: [u8; 32] = [
    159, 101, 165, 10, 103, 89, 107, 118, 134, 35, 62, 205, 14, 245, 130, 168, 86, 190, 41, 247,
    139, 113, 170, 202, 91, 69, 135, 242, 242, 219, 97, 152,
];

pub const ID: Identifier = Identifier(IdentifierBytes32(ID_BYTES));
pub const OWNER_ID: Identifier = Identifier(IdentifierBytes32(OWNER_ID_BYTES));

pub mod document_types {
    pub mod reward_share {
        pub const NAME: &str = "rewardShare";

        pub mod properties {
            pub const PAY_TO_ID: &str = "payToId";
            pub const PERCENTAGE: &str = "percentage";
        }
    }
}

pub fn load_documents_schemas() -> Result<Value, Error> {
    serde_json::from_str(include_str!(
        "../schema/masternode-reward-shares-documents.json"
    ))
}
