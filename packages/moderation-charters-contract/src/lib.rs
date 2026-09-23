mod error;
pub mod v1;

pub use crate::error::Error;
use platform_value::{Identifier, IdentifierBytes32};
use platform_version::version::PlatformVersion;
use serde_json::Value;

pub const ID_BYTES: [u8; 32] = [
    197, 6, 230, 72, 106, 198, 82, 129, 253, 135, 43, 86, 185, 182, 17, 112, 164, 127, 96, 5, 107,
    185, 156, 46, 14, 10, 109, 237, 77, 228, 248, 129,
];

pub const OWNER_ID_BYTES: [u8; 32] = [0; 32];

pub const ID: Identifier = Identifier(IdentifierBytes32(ID_BYTES));
pub const OWNER_ID: Identifier = Identifier(IdentifierBytes32(OWNER_ID_BYTES));

/// The contract's definitions for `platform_version`. Version 0, the value the tables of
/// protocol versions below 14 carry, names no schema generation and is refused like any
/// unknown version: the contract does not exist before its activation.
pub fn load_definitions(platform_version: &PlatformVersion) -> Result<Option<Value>, Error> {
    match platform_version.system_data_contracts.moderation_charters {
        1 => Ok(None),
        version => Err(Error::UnknownVersionMismatch {
            method: "moderation_charters_contract::load_definitions".to_string(),
            known_versions: vec![1],
            received: version,
        }),
    }
}

pub fn load_documents_schemas(platform_version: &PlatformVersion) -> Result<Value, Error> {
    match platform_version.system_data_contracts.moderation_charters {
        1 => v1::load_documents_schemas(),
        version => Err(Error::UnknownVersionMismatch {
            method: "moderation_charters_contract::load_documents_schemas".to_string(),
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

        let base58_decoded = "EG7RGfV8fDTayC2FyVr8HwdpJh3fXDbVztcfE94UmN88"
            .from_base58()
            .unwrap();
        assert_eq!(
            base58_decoded, ID_BYTES,
            "ID should match the base58 decoded value"
        );
    }

    #[test]
    fn should_load_the_schema_at_the_latest_platform_version() {
        let schema = load_documents_schemas(PlatformVersion::latest()).expect("schema loads");
        let charter = schema
            .get(v1::document_types::charter::NAME)
            .expect("the charter document type is declared");
        let properties = charter
            .get("properties")
            .and_then(Value::as_object)
            .expect("the charter has properties");
        for property in [
            v1::document_types::charter::properties::TARGET_CONTRACT_ID,
            v1::document_types::charter::properties::DESCRIPTION,
            v1::document_types::charter::properties::ABILITIES,
            v1::document_types::charter::properties::MEMBERS,
            v1::document_types::charter::properties::REASON_CODES,
            v1::document_types::charter::properties::MODERATORS_SHARE,
            v1::document_types::charter::properties::SPLIT,
        ] {
            assert!(
                properties.contains_key(property),
                "the schema declares {property}"
            );
        }
        assert_eq!(
            charter["indices"][0]["name"],
            v1::document_types::charter::indexes::BY_TARGET_CONTRACT
        );
    }

    #[test]
    fn should_refuse_to_load_below_the_activation_version() {
        let platform_version = PlatformVersion::get(13).expect("protocol version 13");
        assert!(matches!(
            load_documents_schemas(platform_version),
            Err(Error::UnknownVersionMismatch { received: 0, .. })
        ));
        assert!(matches!(
            load_definitions(platform_version),
            Err(Error::UnknownVersionMismatch { received: 0, .. })
        ));
    }
}
