mod error;
pub mod v1;

pub use crate::error::Error;
use platform_value::{Identifier, IdentifierBytes32};
use platform_version::version::PlatformVersion;
use serde_json::Value;

pub const ID_BYTES: [u8; 32] = [
    88, 18, 140, 208, 179, 231, 242, 57, 225, 203, 4, 210, 245, 95, 136, 92, 160, 167, 112, 118,
    173, 238, 83, 62, 234, 230, 222, 16, 231, 30, 99, 98,
];

pub const OWNER_ID_BYTES: [u8; 32] = [0; 32];

pub const ID: Identifier = Identifier(IdentifierBytes32(ID_BYTES));
pub const OWNER_ID: Identifier = Identifier(IdentifierBytes32(OWNER_ID_BYTES));
pub fn load_definitions(platform_version: &PlatformVersion) -> Result<Option<Value>, Error> {
    match platform_version.system_data_contracts.document_history {
        1 => Ok(None),
        version => Err(Error::UnknownVersionMismatch {
            method: "document_history_contract::load_definitions".to_string(),
            known_versions: vec![1],
            received: version,
        }),
    }
}
pub fn load_documents_schemas(platform_version: &PlatformVersion) -> Result<Value, Error> {
    match platform_version.system_data_contracts.document_history {
        1 => v1::load_documents_schemas(),
        version => Err(Error::UnknownVersionMismatch {
            method: "document_history_contract::load_documents_schemas".to_string(),
            known_versions: vec![1],
            received: version,
        }),
    }
}
