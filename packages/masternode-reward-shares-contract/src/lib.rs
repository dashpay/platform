mod error;
pub mod v1;

pub use crate::error::Error;
use platform_value::{Identifier, IdentifierBytes32};
use platform_version::version::PlatformVersion;
use serde_json::Value;

pub const ID_BYTES: [u8; 32] = [
    12, 172, 226, 5, 36, 102, 147, 167, 200, 21, 101, 35, 98, 13, 170, 147, 125, 47, 34, 71, 147,
    68, 99, 238, 176, 31, 247, 33, 149, 144, 149, 140,
];

pub const OWNER_ID_BYTES: [u8; 32] = [0; 32];

pub const ID: Identifier = Identifier(IdentifierBytes32(ID_BYTES));
pub const OWNER_ID: Identifier = Identifier(IdentifierBytes32(OWNER_ID_BYTES));

pub fn load_definitions(platform_version: &PlatformVersion) -> Result<Option<Value>, Error> {
    match platform_version.system_data_contracts.withdrawals {
        0 => Ok(None),
        version => Err(Error::UnknownVersionMismatch {
            method: "masternode_reward_shares_contract::load_definitions".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}
pub fn load_documents_schemas(platform_version: &PlatformVersion) -> Result<Value, Error> {
    match platform_version.system_data_contracts.withdrawals {
        1 => v1::load_documents_schemas(),
        version => Err(Error::UnknownVersionMismatch {
            method: "masternode_reward_shares_contract::load_documents_schemas".to_string(),
            known_versions: vec![1],
            received: version,
        }),
    }
}
