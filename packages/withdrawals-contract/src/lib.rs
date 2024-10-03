pub use crate::error::Error;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use platform_value::{Identifier, IdentifierBytes32};
use platform_version::version::PlatformVersion;
use serde_json::Value;
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::fmt;

mod error;
pub mod v1;

pub const ID_BYTES: [u8; 32] = [
    54, 98, 187, 97, 225, 127, 174, 62, 162, 148, 207, 96, 49, 151, 251, 10, 171, 109, 81, 24, 11,
    216, 182, 16, 76, 73, 68, 166, 47, 226, 217, 127,
];

pub const OWNER_ID_BYTES: [u8; 32] = [0; 32];

pub const ID: Identifier = Identifier(IdentifierBytes32(ID_BYTES));
pub const OWNER_ID: Identifier = Identifier(IdentifierBytes32(OWNER_ID_BYTES));

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
    /// The documents are in the state and waiting to be processed.
    QUEUED = 0,
    /// Pooled happens when we are waiting for signing.
    POOLED = 1,
    /// We have broadcasted the transaction to core.
    BROADCASTED = 2,
    /// The transaction is now complete.
    COMPLETE = 3,
    /// We broadcasted the transaction but core never saw it or rejected it.
    EXPIRED = 4,
}

impl fmt::Display for WithdrawalStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status_str = match self {
            WithdrawalStatus::QUEUED => "Queued",
            WithdrawalStatus::POOLED => "Pooled",
            WithdrawalStatus::BROADCASTED => "Broadcasted",
            WithdrawalStatus::COMPLETE => "Complete",
            WithdrawalStatus::EXPIRED => "Expired",
        };
        write!(f, "{}", status_str)
    }
}

pub fn load_definitions(platform_version: &PlatformVersion) -> Result<Option<Value>, Error> {
    match platform_version.system_data_contracts.withdrawals {
        1 => Ok(None),
        version => Err(Error::UnknownVersionMismatch {
            method: "withdrawals_contract::load_definitions".to_string(),
            known_versions: vec![1],
            received: version,
        }),
    }
}
pub fn load_documents_schemas(platform_version: &PlatformVersion) -> Result<Value, Error> {
    match platform_version.system_data_contracts.withdrawals {
        1 => v1::load_documents_schemas(),
        version => Err(Error::UnknownVersionMismatch {
            method: "withdrawals_contract::load_documents_schemas".to_string(),
            known_versions: vec![1],
            received: version,
        }),
    }
}
