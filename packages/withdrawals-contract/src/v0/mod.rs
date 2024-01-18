use crate::error::Error;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde_json::Value;
use serde_repr::{Deserialize_repr, Serialize_repr};

pub mod document_types {
    pub mod withdrawal {
        pub const NAME: &str = "withdrawal";

        pub mod properties {
            pub const TRANSACTION_ID: &str = "transactionId";
            pub const TRANSACTION_SIGN_HEIGHT: &str = "transactionSignHeight";
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
    EXPIRED = 4,
}

pub(super) fn load_documents_schemas() -> Result<Value, Error> {
    serde_json::from_str(include_str!("../../schema/v0/withdrawals-documents.json"))
        .map_err(Error::InvalidSchemaJson)
}
