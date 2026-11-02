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
    use std::collections::BTreeSet;

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

        let schema = load_documents_schemas(PlatformVersion::latest()).expect("schema loads");
        // One type and no others.
        assert_eq!(schema.as_object().map(|types| types.len()), Some(1));
        let proposal_type = &schema[proposal::NAME];
        let properties = proposal_type["properties"]
            .as_object()
            .expect("proposal has properties");
        let declared: BTreeSet<&str> = properties.keys().map(String::as_str).collect();
        assert_eq!(
            declared,
            BTreeSet::from([
                proposal::properties::TITLE,
                proposal::properties::DESCRIPTION,
                proposal::properties::URL,
                proposal::properties::PASS_PERCENT,
                proposal::properties::MINIMUM_TURNOUT_PERCENT,
                proposal::properties::END_DATE,
            ])
        );
        // A pass threshold of 50 to 80 percent, a turnout of at most half the network
        let bounds = |property: &str| {
            (
                properties[property]["minimum"].as_u64(),
                properties[property]["maximum"].as_u64(),
            )
        };
        assert_eq!(
            bounds(proposal::properties::PASS_PERCENT),
            (Some(50), Some(80))
        );
        assert_eq!(
            bounds(proposal::properties::MINIMUM_TURNOUT_PERCENT),
            (Some(0), Some(50))
        );
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
        // The rule masternodes vote by never changes; the text may be edited for a while, and
        // every version is kept. A proposal is never deleted.
        assert_eq!(proposal_type["documentsMutable"], Value::Bool(true));
        assert_eq!(proposal_type["documentsKeepHistory"], Value::Bool(true));
        assert_eq!(proposal_type["canBeDeleted"], Value::Bool(false));
        let immutable = proposal_type["immutable"]
            .as_array()
            .expect("proposal lists immutable properties");
        for property in [
            proposal::properties::PASS_PERCENT,
            proposal::properties::MINIMUM_TURNOUT_PERCENT,
            proposal::properties::END_DATE,
        ] {
            assert!(
                immutable.contains(&Value::String(property.to_string())),
                "{property} never changes"
            );
        }
        for property in [
            proposal::properties::TITLE,
            proposal::properties::DESCRIPTION,
            proposal::properties::URL,
        ] {
            assert!(
                immutable
                    .iter()
                    .any(|entry| entry["property"] == property && entry["when"].is_object()),
                "{property} freezes once its edit window ends"
            );
        }
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
