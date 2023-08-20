use platform_value::{Identifier, IdentifierBytes32};
use serde_json::Error;
use serde_json::Value;

pub const ID_BYTES: [u8; 32] = [
    162, 161, 180, 172, 111, 239, 34, 234, 42, 26, 104, 232, 18, 54, 68, 179, 87, 135, 95, 107, 65,
    44, 24, 16, 146, 129, 193, 70, 231, 178, 113, 188,
];

pub const OWNER_ID_BYTES: [u8; 32] = [
    65, 63, 57, 243, 204, 9, 106, 71, 187, 2, 94, 221, 190, 127, 141, 114, 137, 209, 243, 50, 60,
    215, 90, 101, 229, 15, 115, 5, 44, 117, 182, 217,
];

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

pub const ID: Identifier = Identifier(IdentifierBytes32(ID_BYTES));
pub const OWNER_ID: Identifier = Identifier(IdentifierBytes32(OWNER_ID_BYTES));

pub fn load_documents_schemas() -> Result<Value, Error> {
    serde_json::from_str(include_str!("../schema/dashpay.schema.json"))
}
