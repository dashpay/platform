//! A summed property is never transient (generation 4, protocol version 15): Drive writes a
//! document's value of the property into the type's sums, and a transient value is never
//! stored. A registration rule: a stored contract still parses, and protocol version 14
//! registers what it registered.
use super::*;
use crate::data_contract::document_type::accessors::{
    DocumentTypeV0Getters, DocumentTypeV2Getters,
};
use crate::data_contract::errors::DataContractError;
use platform_value::platform_value;

fn parse_at_version(
    schema: Value,
    protocol_version: u32,
    full_validation: bool,
) -> Result<DocumentType, ProtocolError> {
    let platform_version =
        PlatformVersion::get(protocol_version).expect("expected platform version");
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("default config available");
    DocumentType::try_from_schema(
        Identifier::new([1; 32]),
        1,
        config.version(),
        "tip",
        schema,
        None,
        &BTreeMap::new(),
        &config,
        full_validation,
        &mut vec![],
        platform_version,
    )
}

fn latest() -> u32 {
    PlatformVersion::latest().protocol_version
}

/// A `tip` type of `label` and a required `amount` between 0 and 1000, indexed `byLabel`,
/// listing `amount` as transient, with `type_keywords` on the type and `index_keywords` on its
/// index.
fn transient_amount_schema(type_keywords: Value, index_keywords: Value) -> Value {
    let mut index = platform_value!({
        "name": "byLabel",
        "properties": [{ "label": "asc" }],
    });
    for (key, value) in index_keywords
        .into_btree_string_map()
        .expect("expected the index keywords to be a map")
    {
        index
            .insert(key, value)
            .expect("expected to set the index keyword");
    }
    let mut schema = platform_value!({
        "type": "object",
        "properties": {
            "label": { "type": "string", "maxLength": 20, "position": 0 },
            "amount": { "type": "integer", "minimum": 0, "maximum": 1000, "position": 1 },
        },
        "required": ["label", "amount"],
        "transient": ["amount"],
        "indices": [index],
        "additionalProperties": false,
    });
    for (key, value) in type_keywords
        .into_btree_string_map()
        .expect("expected the type keywords to be a map")
    {
        schema
            .insert(key, value)
            .expect("expected to set the type keyword");
    }
    schema
}

/// Every way a type declares the property it sums.
fn summing_declarations() -> Vec<(&'static str, Value, Value)> {
    vec![
        (
            "documentsSummable",
            platform_value!({ "documentsSummable": "amount" }),
            platform_value!({}),
        ),
        (
            "documentsAverageable",
            platform_value!({ "documentsAverageable": "amount" }),
            platform_value!({}),
        ),
        (
            "an index's summable",
            platform_value!({}),
            platform_value!({ "summable": "amount" }),
        ),
        (
            "an index's averageable",
            platform_value!({}),
            platform_value!({ "averageable": "amount" }),
        ),
    ]
}

/// A signed contract summing a transient property must be a paid rejection with a nonce
/// bump, so the refusal is the consensus variant and names the property.
#[test]
fn should_refuse_a_summed_property_listed_as_transient() {
    for (declaration, type_keywords, index_keywords) in summing_declarations() {
        let error = parse_at_version(
            transient_amount_schema(type_keywords, index_keywords),
            latest(),
            true,
        )
        .expect_err("a type summing a transient property must be refused");
        let ProtocolError::ConsensusError(consensus) = &error else {
            panic!("{declaration}: expected a consensus error, got {error:?}");
        };
        assert!(
            matches!(
                consensus.as_ref(),
                crate::consensus::ConsensusError::BasicError(
                    crate::consensus::basic::BasicError::ContractError(
                        DataContractError::InvalidContractStructure(message)
                    )
                ) if message.contains("sums \"amount\"") && message.contains("transient")
            ),
            "{declaration}: expected an invalid-contract-structure error naming the property, \
             got {consensus:?}"
        );
    }
}

/// A type that sums a property and lists another as transient is unaffected.
#[test]
fn should_register_a_summed_property_beside_a_transient_one() {
    let schema = platform_value!({
        "type": "object",
        "documentsSummable": "amount",
        "properties": {
            "amount": { "type": "integer", "minimum": 0, "maximum": 1000, "position": 0 },
            "note": { "type": "string", "maxLength": 20, "position": 1 },
        },
        "required": ["amount"],
        "transient": ["note"],
        "additionalProperties": false,
    });
    let document_type = parse_at_version(schema, latest(), true)
        .expect("a summed property that is not transient registers");
    assert_eq!(document_type.documents_summable(), Some("amount"));
    assert!(document_type.transient_fields().contains("note"));
}

/// The rule is a registration rule: a contract stored with such a type, parsed without full
/// validation, still loads.
#[test]
fn should_load_a_stored_type_summing_a_transient_property() {
    for (declaration, type_keywords, index_keywords) in summing_declarations() {
        parse_at_version(
            transient_amount_schema(type_keywords, index_keywords),
            latest(),
            false,
        )
        .unwrap_or_else(|error| panic!("{declaration}: a stored type must load: {error:?}"));
    }
}

/// Protocol version 14 keeps registering what it registered.
#[test]
fn should_keep_registering_a_summed_transient_property_at_protocol_version_14() {
    for (declaration, type_keywords, index_keywords) in summing_declarations() {
        parse_at_version(
            transient_amount_schema(type_keywords, index_keywords),
            14,
            true,
        )
        .unwrap_or_else(|error| {
            panic!("{declaration}: protocol version 14 must still register the type: {error:?}")
        });
    }
}
