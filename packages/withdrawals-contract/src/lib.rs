use num_enum::{IntoPrimitive, TryFromPrimitive};
use platform_value::{Identifier, IdentifierBytes32};
use serde_json::Error;
use serde_json::Value;
use serde_repr::{Deserialize_repr, Serialize_repr};

pub mod document_types {
    pub mod withdrawal {
        pub const NAME: &str = "withdrawal";

        pub mod properties {
            pub const TRANSACTION_INDEX: &str = "transactionIndex";
            pub const AMOUNT: &str = "amount";
            pub const CORE_FEE_PER_BYTE: &str = "coreFeePerByte";
            pub const POOLING: &str = "pooling";
            pub const OUTPUT_SCRIPT: &str = "outputScript";
            pub const STATUS: &str = "status";
            pub const CREATED_AT: &str = "$createdAt";
            pub const UPDATED_AT: &str = "$updatedAt";
            pub const OWNER_ID: &str = "$ownerId";
        }
    }
}

// @append_only
#[repr(u8)]
#[derive(
    Serialize_repr,
    Deserialize_repr,
    PartialEq,
    Eq,
    Clone,
    Copy,
    Debug,
    TryFromPrimitive,
    IntoPrimitive,
)]
pub enum WithdrawalStatus {
    QUEUED = 0,
    POOLED = 1,
    BROADCASTED = 2,
    COMPLETE = 3,
}

pub const ID_BYTES: [u8; 32] = [
    54, 98, 187, 97, 225, 127, 174, 62, 162, 148, 207, 96, 49, 151, 251, 10, 171, 109, 81, 24, 11,
    216, 182, 16, 76, 73, 68, 166, 47, 226, 217, 127,
];

pub const OWNER_ID_BYTES: [u8; 32] = [
    170, 138, 235, 213, 173, 122, 202, 36, 243, 48, 61, 185, 146, 50, 146, 255, 194, 133, 221, 176,
    188, 82, 144, 69, 234, 198, 106, 35, 245, 167, 46, 192,
];

pub const ID: Identifier = Identifier(IdentifierBytes32(ID_BYTES));
pub const OWNER_ID: Identifier = Identifier(IdentifierBytes32(OWNER_ID_BYTES));

pub fn load_documents_schemas() -> Result<Value, Error> {
    serde_json::from_str(include_str!("../schema/withdrawals-documents.json"))
}
