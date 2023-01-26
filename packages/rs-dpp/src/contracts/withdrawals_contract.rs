use lazy_static::lazy_static;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::prelude::Identifier;

pub mod types {
    pub const WITHDRAWAL: &str = "withdrawal";
}

pub mod property_names {
    pub const TRANSACTION_ID: &str = "transactionId";
    pub const TRANSACTION_SIGN_HEIGHT: &str = "transactionSignHeight";
    pub const TRANSACTION_INDEX: &str = "transactionIndex";
    pub const AMOUNT: &str = "amount";
    pub const CORE_FEE_PER_BYTE: &str = "coreFeePerByte";
    pub const POOLING: &str = "pooling";
    pub const OUTPUT_SCRIPT: &str = "outputScript";
    pub const STATUS: &str = "status";
    pub const CREATE_AT: &str = "$createdAt";
}

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
pub enum Status {
    QUEUED = 0,
    POOLED = 1,
    BROADCASTED = 2,
    COMPLETE = 3,
    EXPIRED = 4,
}

lazy_static! {
    pub static ref CONTRACT_ID: Identifier = Identifier::new([
        54, 98, 187, 97, 225, 127, 174, 62, 162, 148, 207, 96, 49, 151, 251, 10, 171, 109, 81, 24,
        11, 216, 182, 16, 76, 73, 68, 166, 47, 226, 217, 127
    ]);
    pub static ref OWNER_ID: Identifier = Identifier::new([
        170, 138, 235, 213, 173, 122, 202, 36, 243, 48, 61, 185, 146, 50, 146, 255, 194, 133, 221,
        176, 188, 82, 144, 69, 234, 198, 106, 35, 245, 167, 46, 192
    ]);
}
