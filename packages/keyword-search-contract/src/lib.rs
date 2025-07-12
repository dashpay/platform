mod error;
pub mod v1;

pub use crate::error::Error;
use platform_value::{Identifier, IdentifierBytes32};
use platform_version::version::PlatformVersion;
use serde_json::Value;

pub const ID_BYTES: [u8; 32] = [
    161, 147, 167, 153, 40, 225, 219, 101, 50, 156, 28, 146, 150, 52, 114, 213, 56, 154, 106, 15,
    79, 66, 18, 156, 94, 146, 216, 104, 140, 93, 170, 215,
];

pub const OWNER_ID_BYTES: [u8; 32] = [0; 32];

pub const ID: Identifier = Identifier(IdentifierBytes32(ID_BYTES));
pub const OWNER_ID: Identifier = Identifier(IdentifierBytes32(OWNER_ID_BYTES));
pub fn load_definitions(platform_version: &PlatformVersion) -> Result<Option<Value>, Error> {
    match platform_version.system_data_contracts.keyword_search {
        1 => Ok(None),
        version => Err(Error::UnknownVersionMismatch {
            method: "keyword_search_contract::load_definitions".to_string(),
            known_versions: vec![1],
            received: version,
        }),
    }
}
pub fn load_documents_schemas(platform_version: &PlatformVersion) -> Result<Value, Error> {
    match platform_version.system_data_contracts.keyword_search {
        1 => v1::load_documents_schemas(),
        version => Err(Error::UnknownVersionMismatch {
            method: "keyword_search_contract::load_documents_schemas".to_string(),
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
    fn test_id() {
        assert_eq!(
            ID,
            Identifier(IdentifierBytes32(ID_BYTES)),
            "ID should match the expected value"
        );

        let base58_decoded = "BsjE6tQxG47wffZCRQCovFx5rYrAYYC3rTVRWKro27LA"
            .from_base58()
            .unwrap();
        assert_eq!(
            base58_decoded, ID_BYTES,
            "ID should match the base58 decoded value"
        );
    }
}
