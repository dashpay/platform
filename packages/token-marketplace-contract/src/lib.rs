mod error;
pub mod v1;

pub use crate::error::Error;
use platform_value::{Identifier, IdentifierBytes32};
use platform_version::version::PlatformVersion;
use serde_json::Value;

pub const ID_BYTES: [u8; 32] = [
    163, 48, 89, 76, 126, 133, 229, 211, 173, 53, 218, 112, 77, 231, 72, 107, 107, 127, 225, 220,
    84, 165, 134, 59, 133, 4, 111, 198, 239, 243, 236, 179,
];

pub const OWNER_ID_BYTES: [u8; 32] = [0; 32];

pub const ID: Identifier = Identifier(IdentifierBytes32(ID_BYTES));
pub const OWNER_ID: Identifier = Identifier(IdentifierBytes32(OWNER_ID_BYTES));
pub fn load_definitions(platform_version: &PlatformVersion) -> Result<Option<Value>, Error> {
    match platform_version.system_data_contracts.token_marketplace {
        1 => Ok(None),
        version => Err(Error::UnknownVersionMismatch {
            method: "token_marketplace::load_definitions".to_string(),
            known_versions: vec![1],
            received: version,
        }),
    }
}
pub fn load_documents_schemas(platform_version: &PlatformVersion) -> Result<Value, Error> {
    match platform_version.system_data_contracts.token_marketplace {
        1 => v1::load_documents_schemas(),
        version => Err(Error::UnknownVersionMismatch {
            method: "token_marketplace::load_documents_schemas".to_string(),
            known_versions: vec![1],
            received: version,
        }),
    }
}
