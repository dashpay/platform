mod error;
pub mod v1;

pub use crate::error::Error;
use platform_value::{Identifier, IdentifierBytes32};
use platform_version::version::PlatformVersion;
use serde_json::Value;

pub const ID_BYTES: [u8; 32] = [
    137, 20, 125, 33, 9, 113, 160, 140, 158, 199, 2, 183, 75, 230, 71, 38, 213, 195, 65, 13, 129,
    32, 10, 91, 226, 116, 230, 164, 77, 101, 0, 147,
];

pub const OWNER_ID_BYTES: [u8; 32] = [0; 32];

pub const ID: Identifier = Identifier(IdentifierBytes32(ID_BYTES));
pub const OWNER_ID: Identifier = Identifier(IdentifierBytes32(OWNER_ID_BYTES));

/// The contract's definitions for `platform_version`. Version 0, the value the tables of
/// protocol versions below 15 carry, names no schema generation and is refused like any
/// unknown version: the contract does not exist before its activation.
pub fn load_definitions(platform_version: &PlatformVersion) -> Result<Option<Value>, Error> {
    match platform_version.system_data_contracts.token_market {
        1 => Ok(None),
        version => Err(Error::UnknownVersionMismatch {
            method: "token_market_contract::load_definitions".to_string(),
            known_versions: vec![1],
            received: version,
        }),
    }
}

pub fn load_documents_schemas(platform_version: &PlatformVersion) -> Result<Value, Error> {
    match platform_version.system_data_contracts.token_market {
        1 => v1::load_documents_schemas(),
        version => Err(Error::UnknownVersionMismatch {
            method: "token_market_contract::load_documents_schemas".to_string(),
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
        let base58_decoded = "AE6yYq65VdfGCg2MSyV9vV7cWGiFWjbwBuy7THesDh4e"
            .from_base58()
            .expect("expected a base58 id");
        assert_eq!(base58_decoded, ID_BYTES);
    }

    #[test]
    fn should_load_the_schema_at_the_latest_platform_version() {
        use v1::document_types::{ask, bid};

        let schema = load_documents_schemas(PlatformVersion::latest()).expect("schema loads");
        let declared = |name: &str, properties: &[&str], summed: &str| {
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
            let indexes = document_type
                .get("indices")
                .and_then(Value::as_array)
                .unwrap_or_else(|| panic!("{name} has indexes"));
            let declared_indexes: Vec<&str> = indexes
                .iter()
                .filter_map(|index| index.get("name").and_then(Value::as_str))
                .collect();
            assert_eq!(
                declared_indexes,
                [
                    ask::indexes::BOOK,
                    ask::indexes::DEPTH,
                    ask::indexes::BY_OWNER
                ],
                "the indexes of {name}"
            );
            assert_eq!(
                indexes[1].get("summable").and_then(Value::as_str),
                Some(summed),
                "the depth index of {name} sums {summed}"
            );
            // Only the platform writes orders.
            assert_eq!(document_type["creationRestrictionMode"], 2);
        };

        declared(
            ask::NAME,
            &[
                ask::properties::TOKEN_ID,
                ask::properties::PRICE,
                ask::properties::AMOUNT,
                ask::properties::REMAINING,
                ask::properties::MIN_FILL,
                ask::properties::EXPIRES_AT,
            ],
            ask::properties::REMAINING,
        );
        declared(
            bid::NAME,
            &[
                bid::properties::TOKEN_ID,
                bid::properties::PRICE,
                bid::properties::AMOUNT,
                bid::properties::REMAINING,
                bid::properties::MIN_FILL,
                bid::properties::EXPIRES_AT,
                bid::properties::ESCROW,
            ],
            bid::properties::ESCROW,
        );
        assert_eq!(schema.as_object().map(|types| types.len()), Some(2));
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
