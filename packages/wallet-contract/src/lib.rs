mod error;
pub mod v1;

pub use crate::error::Error;
use platform_value::{Identifier, IdentifierBytes32};
use platform_version::version::PlatformVersion;
use serde_json::Value;

pub const ID_BYTES: [u8; 32] = [
    162, 48, 73, 255, 116, 241, 166, 155, 131, 121, 132, 39, 129, 40, 127, 6, 103, 164, 72, 139,
    143, 116, 163, 19, 81, 193, 38, 248, 116, 244, 59, 196
];

pub const OWNER_ID_BYTES: [u8; 32] = [0; 32];

pub const ID: Identifier = Identifier(IdentifierBytes32(ID_BYTES));
pub const OWNER_ID: Identifier = Identifier(IdentifierBytes32(OWNER_ID_BYTES));
pub fn load_definitions(platform_version: &PlatformVersion) -> Result<Option<Value>, Error> {
    match platform_version.system_data_contracts.withdrawals {
        1 => Ok(None),
        version => Err(Error::UnknownVersionMismatch {
            method: "wallet_contract::load_definitions".to_string(),
            known_versions: vec![1],
            received: version,
        }),
    }
}
pub fn load_documents_schemas(platform_version: &PlatformVersion) -> Result<Value, Error> {
    match platform_version.system_data_contracts.withdrawals {
        1 => v1::load_documents_schemas(),
        version => Err(Error::UnknownVersionMismatch {
            method: "wallet_contract::load_documents_schemas".to_string(),
            known_versions: vec![1],
            received: version,
        }),
    }
}
