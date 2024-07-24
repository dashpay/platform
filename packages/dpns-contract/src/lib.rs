mod error;
pub mod v1;

pub use crate::error::Error;
use platform_value::{Identifier, IdentifierBytes32};
use platform_version::version::PlatformVersion;
use serde_json::Value;

pub const ID_BYTES: [u8; 32] = [
    230, 104, 198, 89, 175, 102, 174, 225, 231, 44, 24, 109, 222, 123, 91, 126, 10, 29, 113, 42, 9,
    196, 13, 87, 33, 246, 34, 191, 83, 197, 49, 85,
];

pub const OWNER_ID_BYTES: [u8; 32] = [0; 32];

pub const DPNS_DASH_TLD_DOCUMENT_ID: [u8; 32] = [
    215, 242, 197, 63, 70, 169, 23, 171, 110, 91, 57, 162, 215, 188, 38, 11, 100, 146, 137, 69, 55,
    68, 209, 224, 212, 242, 106, 141, 142, 255, 55, 207,
];

pub const DPNS_DASH_TLD_PREORDER_SALT: [u8; 32] = [
    224, 181, 8, 197, 163, 104, 37, 162, 6, 105, 58, 31, 65, 74, 161, 62, 219, 236, 244, 60, 65,
    227, 199, 153, 234, 158, 115, 123, 79, 154, 162, 38,
];

pub const ID: Identifier = Identifier(IdentifierBytes32(ID_BYTES));
pub const OWNER_ID: Identifier = Identifier(IdentifierBytes32(OWNER_ID_BYTES));
pub fn load_definitions(platform_version: &PlatformVersion) -> Result<Option<Value>, Error> {
    match platform_version.system_data_contracts.withdrawals {
        1 => Ok(None),
        version => Err(Error::UnknownVersionMismatch {
            method: "dpns_contract::load_definitions".to_string(),
            known_versions: vec![1],
            received: version,
        }),
    }
}
pub fn load_documents_schemas(platform_version: &PlatformVersion) -> Result<Value, Error> {
    match platform_version.system_data_contracts.withdrawals {
        1 => v1::load_documents_schemas(),
        version => Err(Error::UnknownVersionMismatch {
            method: "dpns_contract::load_documents_schemas".to_string(),
            known_versions: vec![1],
            received: version,
        }),
    }
}
