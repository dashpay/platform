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
        use v1::document_types::{
            added_moderator, elected_charter, join_request, reason, removed_moderator,
            resignation_request, submitted_charter,
        };

        let schema = load_documents_schemas(PlatformVersion::latest()).expect("schema loads");
        let declared = |name: &str, properties: &[&str], indexes: &[&str]| {
            let document_type = schema
                .get(name)
                .unwrap_or_else(|| panic!("the {name} document type is declared"));
            let declared_properties = document_type
                .get("properties")
                .and_then(Value::as_object)
                .unwrap_or_else(|| panic!("{name} has properties"));
            for property in properties {
                assert!(
                    declared_properties.contains_key(*property),
                    "{name} declares {property}"
                );
            }
            let declared_indexes: Vec<&str> = document_type
                .get("indices")
                .and_then(Value::as_array)
                .unwrap_or_else(|| panic!("{name} has indexes"))
                .iter()
                .filter_map(|index| index.get("name").and_then(Value::as_str))
                .collect();
            assert_eq!(declared_indexes, indexes, "the indexes of {name}");
        };

        declared(
            reason::NAME,
            &[
                reason::properties::CODE,
                reason::properties::LABEL,
                reason::properties::DESCRIPTION,
            ],
            &[reason::indexes::BY_OWNER_CODE],
        );
        declared(
            submitted_charter::NAME,
            &[
                submitted_charter::properties::TARGET_CONTRACT_ID,
                submitted_charter::properties::DESCRIPTION,
                submitted_charter::properties::REASONS,
                submitted_charter::properties::MODERATORS_SHARE,
                submitted_charter::properties::REWARD_SPLIT,
            ],
            &[
                submitted_charter::indexes::BY_TARGET_CONTRACT,
                submitted_charter::indexes::BY_OWNER,
            ],
        );
        declared(
            join_request::NAME,
            &[
                join_request::properties::SUBMITTED_CHARTER_ID,
                join_request::properties::RECIPIENT_ID,
                join_request::properties::RECIPIENT_KEY_ID,
                join_request::properties::SENDER_KEY_ID,
                join_request::properties::ENCRYPTED_MESSAGE,
            ],
            &[
                join_request::indexes::BY_SUBMITTED_CHARTER,
                join_request::indexes::BY_OWNER,
            ],
        );
        declared(
            elected_charter::NAME,
            &[
                elected_charter::properties::TARGET_CONTRACT_ID,
                elected_charter::properties::SUBMITTED_CHARTER_ID,
                elected_charter::properties::MEMBERS,
            ],
            &[
                elected_charter::indexes::BY_TARGET_CONTRACT,
                elected_charter::indexes::BY_SUBMITTED_CHARTER,
            ],
        );
        declared(
            added_moderator::NAME,
            &[
                added_moderator::properties::ELECTED_CHARTER_ID,
                added_moderator::properties::SUBMITTED_CHARTER_ID,
                added_moderator::properties::MEMBER_ID,
            ],
            &[added_moderator::indexes::BY_ELECTED_CHARTER_MEMBER],
        );
        declared(
            removed_moderator::NAME,
            &[
                removed_moderator::properties::ELECTED_CHARTER_ID,
                removed_moderator::properties::MEMBER_ID,
            ],
            &[removed_moderator::indexes::BY_ELECTED_CHARTER_MEMBER],
        );
        declared(
            resignation_request::NAME,
            &[
                resignation_request::properties::ELECTED_CHARTER_ID,
                resignation_request::properties::RECIPIENT_ID,
                resignation_request::properties::RECIPIENT_KEY_ID,
                resignation_request::properties::SENDER_KEY_ID,
                resignation_request::properties::ENCRYPTED_MESSAGE,
            ],
            &[resignation_request::indexes::BY_ELECTED_CHARTER_OWNER],
        );
        // Seven types and no others.
        assert_eq!(schema.as_object().map(|types| types.len()), Some(7));
        // Only the elected charter is contested, on its target.
        assert!(schema[elected_charter::NAME]["indices"][0]["contested"].is_object());
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
