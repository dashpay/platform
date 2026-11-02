mod error;
pub mod v1;

pub use crate::error::Error;
use platform_value::{Identifier, IdentifierBytes32};
use platform_version::version::PlatformVersion;
use serde_json::Value;

pub const ID_BYTES: [u8; 32] = [
    239, 150, 14, 165, 105, 114, 235, 173, 190, 248, 162, 126, 247, 218, 92, 129, 255, 75, 179,
    138, 2, 150, 151, 69, 126, 36, 218, 66, 183, 155, 84, 183,
];

pub const OWNER_ID_BYTES: [u8; 32] = [0; 32];

pub const ID: Identifier = Identifier(IdentifierBytes32(ID_BYTES));
pub const OWNER_ID: Identifier = Identifier(IdentifierBytes32(OWNER_ID_BYTES));
/// The contract's definitions for `platform_version`. Version 0, the value the tables of
/// protocol versions below 14 carry, names no schema generation and is refused like any
/// unknown version: the contract does not exist before its activation.
pub fn load_definitions(platform_version: &PlatformVersion) -> Result<Option<Value>, Error> {
    match platform_version.system_data_contracts.app_connect {
        1 => Ok(None),
        version => Err(Error::UnknownVersionMismatch {
            method: "app_connect_contract::load_definitions".to_string(),
            known_versions: vec![1],
            received: version,
        }),
    }
}
pub fn load_documents_schemas(platform_version: &PlatformVersion) -> Result<Value, Error> {
    match platform_version.system_data_contracts.app_connect {
        1 => v1::load_documents_schemas(),
        version => Err(Error::UnknownVersionMismatch {
            method: "app_connect_contract::load_documents_schemas".to_string(),
            known_versions: vec![1],
            received: version,
        }),
    }
}

#[cfg(test)]
mod tests {
    use base58::FromBase58;

    use super::*;

    #[test]
    /// Ensure that the ID constant matches the expected value
    /// and that it can be encoded to base58 correctly.
    fn should_match_the_published_system_contract_id() {
        assert_eq!(
            ID,
            Identifier(IdentifierBytes32(ID_BYTES)),
            "ID should match the expected value"
        );

        let base58_decoded = "H8F9mP1BM55TE1ShsxPZHzhyinaMdY9bMmP85mkDhcJJ"
            .from_base58()
            .unwrap();
        assert_eq!(
            base58_decoded, ID_BYTES,
            "ID should match the base58 decoded value"
        );
    }
}
