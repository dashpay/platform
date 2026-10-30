mod error;
pub mod v1;

pub use crate::error::Error;
use platform_value::{Identifier, IdentifierBytes32};
use platform_version::version::PlatformVersion;
use serde_json::Value;

pub const ID_BYTES: [u8; 32] = [
    40, 81, 10, 2, 166, 107, 62, 218, 6, 184, 111, 202, 230, 218, 170, 157, 181, 119, 102, 211,
    195, 114, 92, 200, 90, 88, 205, 30, 174, 49, 203, 200,
];

pub const OWNER_ID_BYTES: [u8; 32] = [0; 32];

pub const ID: Identifier = Identifier(IdentifierBytes32(ID_BYTES));
pub const OWNER_ID: Identifier = Identifier(IdentifierBytes32(OWNER_ID_BYTES));

/// The contract's definitions for `platform_version`. Version 0, the value the tables of
/// protocol versions below 15 carry, names no schema generation and is refused like any
/// unknown version: the contract does not exist before its activation.
pub fn load_definitions(platform_version: &PlatformVersion) -> Result<Option<Value>, Error> {
    match platform_version.system_data_contracts.decision_proposals {
        1 => Ok(None),
        version => Err(Error::UnknownVersionMismatch {
            method: "decision_proposals_contract::load_definitions".to_string(),
            known_versions: vec![1],
            received: version,
        }),
    }
}

pub fn load_documents_schemas(platform_version: &PlatformVersion) -> Result<Value, Error> {
    match platform_version.system_data_contracts.decision_proposals {
        1 => v1::load_documents_schemas(),
        version => Err(Error::UnknownVersionMismatch {
            method: "decision_proposals_contract::load_documents_schemas".to_string(),
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
    fn should_match_the_published_system_contract_id() {
        let base58_decoded = "3iNyoNwjCTkb7XS3RHyWUtLx7pgC6AFAEQNNDHzxqngF"
            .from_base58()
            .expect("base58 id");
        assert_eq!(base58_decoded, ID_BYTES);
        assert_eq!(ID, Identifier(IdentifierBytes32(ID_BYTES)));
    }

    #[test]
    fn should_load_the_schema_at_the_latest_platform_version() {
        use v1::document_types::proposal;
        use v1::document_types::proposal::properties::minimum_share;

        let schema = load_documents_schemas(PlatformVersion::latest()).expect("schema loads");
        // One type and no others.
        assert_eq!(schema.as_object().map(|types| types.len()), Some(1));
        let proposal_type = &schema[proposal::NAME];
        let properties = proposal_type["properties"]
            .as_object()
            .expect("proposal has properties");
        for property in [
            proposal::properties::TITLE,
            proposal::properties::DESCRIPTION,
            proposal::properties::URL,
            proposal::properties::SUPERMAJORITY_NUMERATOR,
            proposal::properties::SUPERMAJORITY_DENOMINATOR,
            proposal::properties::MINIMUM_VOTING_POWER,
            proposal::properties::MINIMUM_SHARE,
            proposal::properties::END_DATE,
        ] {
            assert!(
                properties.contains_key(property),
                "proposal declares {property}"
            );
        }
        let share = &properties[proposal::properties::MINIMUM_SHARE]["properties"];
        for key in [
            minimum_share::NUMERATOR,
            minimum_share::DENOMINATOR,
            minimum_share::ROUNDING,
        ] {
            assert!(share.get(key).is_some(), "minimumShare declares {key}");
        }
        let roundings: Vec<&str> = share[minimum_share::ROUNDING]["enum"]
            .as_array()
            .expect("rounding is an enum")
            .iter()
            .filter_map(Value::as_str)
            .collect();
        assert_eq!(roundings, minimum_share::ROUNDINGS);
        let indexes: Vec<&str> = proposal_type["indices"]
            .as_array()
            .expect("proposal has indexes")
            .iter()
            .filter_map(|index| index["name"].as_str())
            .collect();
        assert_eq!(
            indexes,
            [
                proposal::indexes::BY_END_DATE,
                proposal::indexes::BY_OWNER,
                proposal::indexes::BY_CREATED_AT
            ]
        );
        // What masternodes vote on cannot change after it opens.
        assert_eq!(proposal_type["documentsMutable"], Value::Bool(false));
        assert_eq!(proposal_type["canBeDeleted"], Value::Bool(false));
    }

    #[test]
    fn should_refuse_to_load_below_the_activation_version() {
        let platform_version = PlatformVersion::get(14).expect("protocol version 14");
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
