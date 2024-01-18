pub use crate::error::Error;
use platform_value::{Identifier, IdentifierBytes32};
use platform_version::version::PlatformVersion;
use serde_json::Value;

mod error;
pub mod v0;

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
        1 => v0::load_documents_schemas(),
        version => Err(Error::UnknownVersionMismatch {
            method: "withdrawals_contract::load_documents_schemas".to_string(),
            known_versions: vec![1],
            received: version,
        }),
    }
}
