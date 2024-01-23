mod error;
pub mod v1;

pub use crate::error::Error;
use platform_value::{Identifier, IdentifierBytes32};
use platform_version::version::PlatformVersion;
use serde_json::Value;

pub const ID_BYTES: [u8; 32] = [
    245, 172, 216, 200, 193, 110, 185, 172, 40, 110, 7, 132, 190, 86, 127, 80, 9, 244, 86, 26, 243,
    212, 255, 2, 91, 7, 90, 243, 68, 55, 152, 34,
];

pub const OWNER_ID_BYTES: [u8; 32] = [
    240, 1, 0, 176, 193, 227, 118, 43, 139, 193, 66, 30, 17, 60, 118, 178, 166, 53, 197, 147, 11,
    154, 191, 43, 51, 101, 131, 190, 89, 135, 167, 21,
];

pub const ID: Identifier = Identifier(IdentifierBytes32(ID_BYTES));
pub const OWNER_ID: Identifier = Identifier(IdentifierBytes32(OWNER_ID_BYTES));

pub fn load_definitions(platform_version: &PlatformVersion) -> Result<Option<Value>, Error> {
    match platform_version.system_data_contracts.withdrawals {
        1 => Ok(None),
        version => Err(Error::UnknownVersionMismatch {
            method: "feature_flags_contract::load_definitions".to_string(),
            known_versions: vec![1],
            received: version,
        }),
    }
}
pub fn load_documents_schemas(platform_version: &PlatformVersion) -> Result<Value, Error> {
    match platform_version.system_data_contracts.withdrawals {
        1 => v1::load_documents_schemas(),
        version => Err(Error::UnknownVersionMismatch {
            method: "feature_flags_contract::load_documents_schemas".to_string(),
            known_versions: vec![1],
            received: version,
        }),
    }
}
