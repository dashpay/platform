//! The `propertyConstraints` doctype keyword under generation 3 (protocol
//! version 14): the rules are parsed onto the document type on both paths,
//! every property a rule reads must be a stored integer property, the limits
//! hold under full validation only, the meta-schema checks the grammar when a
//! contract registers, a contract round-trips through platform serialization
//! with its rules, and any change to them is an incompatible update. Protocol
//! version 13 refuses the keyword when registering and ignores it when reading.

use super::immutable_tests::{expect_structure_error, parse_dispatched};
use super::*;
use crate::block::block_info::BlockInfo;
use crate::consensus::basic::basic_error::BasicError;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::document_type::accessors::DocumentTypeV2Getters;
use crate::data_contract::methods::validate_update::DataContractUpdateValidationMethodsV0;
use crate::data_contract::DataContract;
use crate::serialization::{
    PlatformDeserializableWithPotentialValidationFromVersionedStructureUntrusted,
    PlatformSerializableWithPlatformVersion,
};
use platform_value::platform_value;
use platform_value::string_encoding::Encoding;
use serde_json::json;

/// An `order` type: four required integers, an optional nested `meta` object
/// with an integer `total`, and a string, a number, a typed array and an
/// integer `code` to be refused as operands or listed as transient.
fn order_schema(rules: Option<serde_json::Value>, transient: Option<&str>) -> serde_json::Value {
    let mut schema = json!({
        "type": "object",
        "documentsMutable": true,
        "properties": {
            "price": { "type": "integer", "minimum": 0, "maximum": 1000000, "position": 0 },
            "fee": { "type": "integer", "minimum": 0, "maximum": 1000, "position": 1 },
            "quantity": { "type": "integer", "minimum": 1, "maximum": 100, "position": 2 },
            "deposit": { "type": "integer", "minimum": 0, "position": 3 },
            "note": { "type": "string", "maxLength": 63, "position": 4 },
            "ratio": { "type": "number", "position": 5 },
            "meta": {
                "type": "object",
                "position": 6,
                "properties": {
                    "total": { "type": "integer", "minimum": 0, "position": 0 },
                    "tag": { "type": "string", "maxLength": 30, "position": 1 }
                },
                "additionalProperties": false
            },
            "code": { "type": "integer", "minimum": 0, "maximum": 9, "position": 7 },
            "counts": {
                "type": "array",
                "items": { "type": "integer", "minimum": 0, "maximum": 10 },
                "maxItems": 4,
                "position": 8
            }
        },
        "required": ["price", "fee", "quantity", "deposit"],
        "additionalProperties": false
    });
    if let Some(rules) = rules {
        schema["propertyConstraints"] = rules;
    }
    if let Some(transient) = transient {
        schema["transient"] = json!([transient]);
    }
    schema
}

fn schema_value(schema: serde_json::Value) -> Value {
    platform_value::to_value(schema).expect("the schema converts")
}

fn parse_order(
    rules: serde_json::Value,
    full_validation: bool,
) -> Result<DocumentType, ProtocolError> {
    parse_dispatched(
        schema_value(order_schema(Some(rules), None)),
        PlatformVersion::latest(),
        full_validation,
    )
}

fn deposit_rule() -> serde_json::Value {
    json!({
        "lessThanOrEqual": [
            { "multiply": [{ "add": ["price", "fee"] }, "quantity"] },
            "deposit"
        ]
    })
}

fn is_json_schema_error(error: &ProtocolError) -> bool {
    matches!(
        error,
        ProtocolError::ConsensusError(boxed)
            if matches!(**boxed, ConsensusError::BasicError(BasicError::JsonSchemaError(_)))
    )
}

#[test]
fn should_parse_the_rules_onto_the_document_type_on_both_paths() {
    let rules = json!({
        "depositCoversOrder": deposit_rule(),
        "metaWithinDeposit": { "lessThanOrEqual": [{ "ifAbsent": ["meta.total", 0] }, "deposit"] }
    });
    for full_validation in [true, false] {
        let document_type = parse_order(rules.clone(), full_validation)
            .unwrap_or_else(|e| panic!("full_validation {full_validation}: should parse: {e}"));
        let constraints = document_type.property_constraints();
        assert_eq!(
            constraints.keys().collect::<Vec<_>>(),
            ["depositCoversOrder", "metaWithinDeposit"]
        );
        assert_eq!(
            constraints["depositCoversOrder"].property_paths(),
            ["price", "fee", "quantity", "deposit"]
        );
        assert_eq!(
            constraints["metaWithinDeposit"].property_paths(),
            ["meta.total", "deposit"]
        );
    }

    // A type without the keyword has no rules
    let document_type = parse_dispatched(
        schema_value(order_schema(None, None)),
        PlatformVersion::latest(),
        true,
    )
    .expect("parses");
    assert!(document_type.property_constraints().is_empty());
}

/// Only an integer property's value is a number the rule can compute with: a
/// string, a float, an array, an object and a system property are refused on
/// both paths, as is a path naming nothing.
#[test]
fn should_refuse_a_rule_reading_anything_but_an_integer_property() {
    for (operand, needle) in [
        ("note", "reads \"note\", which has type string, not integer"),
        ("ratio", "reads \"ratio\", which has type f64, not integer"),
        (
            "counts",
            "reads \"counts\", which has type array, not integer",
        ),
        (
            "meta.tag",
            "reads \"meta.tag\", which has type string, not integer",
        ),
        (
            "meta",
            "reads \"meta\", which is not an integer property of the document type",
        ),
        (
            "missing",
            "reads \"missing\", which is not an integer property",
        ),
        (
            "meta.missing",
            "reads \"meta.missing\", which is not an integer property",
        ),
        (
            "$ownerId",
            "reads \"$ownerId\", which is not an integer property",
        ),
    ] {
        for full_validation in [true, false] {
            let result = parse_order(
                json!({ "rule": { "lessThan": [operand, "price"] } }),
                full_validation,
            );
            // The meta-schema refuses a `$` in a path when registering
            if full_validation && operand.starts_with('$') {
                assert!(
                    result.as_ref().is_err_and(is_json_schema_error),
                    "{operand}: {result:?}"
                );
                continue;
            }
            expect_structure_error(result, needle);
        }
    }

    // A key id is an integer too
    let schema = platform_value!({
        "type": "object",
        "properties": {
            "recipientId": {
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "maxItems": 32,
                "contentMediaType": "application/x.dash.dpp.identifier",
                "position": 0
            },
            "keyId": {
                "type": "integer",
                "minimum": 0,
                "maximum": 4294967295u32,
                "position": 1,
                "refersTo": {
                    "type": "identityPublicKey",
                    "identityProperty": "$ownerId"
                }
            }
        },
        "propertyConstraints": { "rule": { "greaterThan": ["keyId", 0] } },
        "additionalProperties": false
    });
    parse_dispatched(schema, PlatformVersion::latest(), true)
        .expect("a rule may read a key id property");
}

/// A transient value is never stored, so a stored document could not be held
/// to a rule reading one, whether the property is transient itself or sits in
/// a transient object.
#[test]
fn should_refuse_a_rule_reading_a_transient_value() {
    for (transient, operand) in [("code", "code"), ("meta", "meta.total")] {
        let schema = order_schema(
            Some(json!({ "rule": { "lessThan": [operand, "price"] } })),
            Some(transient),
        );
        for full_validation in [true, false] {
            expect_structure_error(
                parse_dispatched(
                    schema_value(schema.clone()),
                    PlatformVersion::latest(),
                    full_validation,
                ),
                &format!("reads \"{operand}\", which is transient or inside a transient object"),
            );
        }
    }

    // A rule reading a stored property beside a transient one registers
    let schema = order_schema(
        Some(json!({ "rule": { "lessThan": ["price", 1000] } })),
        Some("code"),
    );
    parse_dispatched(schema_value(schema), PlatformVersion::latest(), true)
        .expect("a rule over a stored property registers");
}

#[test]
fn should_hold_the_limits_under_full_validation_only() {
    let limits = &PlatformVersion::latest().system_limits;
    let max_rules = usize::from(limits.max_property_constraints);
    let max_nodes = usize::from(limits.max_property_constraint_nodes);

    let rules = |count: usize| {
        serde_json::Value::Object(
            (0..count)
                .map(|index| {
                    (
                        format!("rule{index:02}"),
                        json!({ "lessThanOrEqual": ["price", 1000000] }),
                    )
                })
                .collect(),
        )
    };
    parse_order(rules(max_rules), true).expect("the most rules a type may declare");
    expect_structure_error(
        parse_order(rules(max_rules + 1), true),
        &format!(
            "declares {} rules, above the maximum of {max_rules}",
            max_rules + 1
        ),
    );
    parse_order(rules(max_rules + 1), false).expect("a stored contract stays readable");

    // equal, add, "price" and ones, 0: the add takes the nodes the rule has left
    let rule_of = |nodes: usize| {
        let mut operands = vec![json!("price")];
        operands.resize(nodes - 3, json!(1));
        json!({ "rule": { "equal": [{ "add": operands }, 0] } })
    };
    let document_type =
        parse_order(rule_of(max_nodes), true).expect("the most nodes a rule may have");
    assert_eq!(
        document_type.property_constraints()["rule"].node_count(),
        max_nodes
    );
    expect_structure_error(
        parse_order(rule_of(max_nodes + 1), true),
        &format!(
            "rule \"rule\" has {} nodes, above the maximum of {max_nodes}",
            max_nodes + 1
        ),
    );
    parse_order(rule_of(max_nodes + 1), false).expect("a stored contract stays readable");
}

/// When a contract registers, the meta-schema checks the grammar, the
/// `ifAbsent` pair included; a stored contract is never validated against it,
/// and the parser refuses the same shapes there.
#[test]
fn should_check_the_grammar_with_the_meta_schema_and_the_parser() {
    for rules in [
        json!({}),
        json!({ "rule": { "equal": ["price"] } }),
        json!({ "rule": { "equal": ["price", 1, 2] } }),
        json!({ "rule": { "atMost": ["price", 1] } }),
        json!({ "rule": { "equal": ["price", 1], "lessThan": ["price", 1] } }),
        json!({ "rule": { "equal": ["price", true] } }),
        json!({ "rule": { "equal": ["price", 1.5] } }),
        json!({ "rule": { "equal": [{ "add": ["price"] }, 1] } }),
        json!({ "rule": { "equal": [{ "subtract": ["price", 1, 2] }, 1] } }),
        json!({ "rule": { "equal": [{ "sum": ["price", 1] }, 1] } }),
        json!({ "rule": { "equal": [{ "add": ["price", 1], "subtract": ["price", 1] }, 1] } }),
        json!({ "rule": { "equal": [{ "ifAbsent": ["price"] }, 1] } }),
        json!({ "rule": { "equal": [{ "ifAbsent": ["price", 1, 2] }, 1] } }),
        json!({ "rule": { "equal": [{ "ifAbsent": [1, "price"] }, 1] } }),
        json!({ "rule": { "equal": [{ "ifAbsent": ["price", "fee"] }, 1] } }),
        json!({ "bad-name": { "equal": ["price", 1] } }),
        json!(["price"]),
    ] {
        let registered = parse_order(rules.clone(), true);
        assert!(
            registered.as_ref().is_err_and(is_json_schema_error),
            "{rules}: the meta-schema should refuse it, got {registered:?}"
        );
        expect_structure_error(parse_order(rules, false), "propertyConstraints");
    }

    // What the meta-schema cannot see, the parser refuses on both paths
    for (rules, needle) in [
        (
            json!({ "rule": { "equal": [{ "divide": ["price", 0] }, 1] } }),
            "at equal[0].divide divides by 0",
        ),
        (
            json!({ "rule": { "equal": [{ "modulo": ["price", 0] }, 1] } }),
            "at equal[0].modulo divides by 0",
        ),
        (
            json!({ "rule": { "equal": [{ "power": ["price", -1] }, 1] } }),
            "at equal[0].power raises to the negative power -1",
        ),
        (
            json!({ "rule": { "equal": [{ "add": [1, 2] }, 3] } }),
            "rule \"rule\" reads no property",
        ),
    ] {
        for full_validation in [true, false] {
            expect_structure_error(parse_order(rules.clone(), full_validation), needle);
        }
    }
}

#[test]
fn should_refuse_property_constraints_before_protocol_version_14_and_ignore_them_when_reading() {
    let mut schema = order_schema(Some(json!({ "depositCoversOrder": deposit_rule() })), None);
    // Typed arrays arrived with protocol version 14 as well
    schema["properties"]
        .as_object_mut()
        .expect("the properties")
        .remove("counts");
    let schema = schema_value(schema);
    let platform_version_13 = PlatformVersion::get(13).expect("protocol version 13");

    // Meta-schema v2 closes the document type level, so a registering parse
    // refuses the unknown keyword
    let registered = parse_dispatched(schema.clone(), platform_version_13, true);
    assert!(
        registered.as_ref().is_err_and(is_json_schema_error),
        "{registered:?}"
    );
    // A parser predating it does not read it
    let read = parse_dispatched(schema.clone(), platform_version_13, false)
        .expect("protocol version 13 reads the rest of the type");
    assert!(read.property_constraints().is_empty());

    let parsed = parse_dispatched(schema, PlatformVersion::latest(), true).expect("parses");
    assert_eq!(parsed.property_constraints().len(), 1);
}

const CONTRACT_ID: [u8; 32] = [7; 32];

fn order_contract(rules: Option<serde_json::Value>, version: u32) -> DataContract {
    let contract = json!({
        "$formatVersion": "1",
        "id": Identifier::from(CONTRACT_ID).to_string(Encoding::Base58),
        "ownerId": Identifier::from([8; 32]).to_string(Encoding::Base58),
        "version": version,
        "documentSchemas": { "order": order_schema(rules, None) }
    });
    DataContract::from_value(
        platform_value::to_value(contract).expect("the contract converts"),
        true,
        PlatformVersion::latest(),
    )
    .expect("the contract parses")
}

/// The rules ride on the schema, which is what the contract serializes: a
/// contract comes back with the same rules parsed from it.
#[test]
fn should_round_trip_a_contract_with_its_rules_through_platform_serialization() {
    let platform_version = PlatformVersion::latest();
    let original = order_contract(Some(json!({ "depositCoversOrder": deposit_rule() })), 1);
    let bytes = original
        .serialize_to_bytes_with_platform_version(platform_version)
        .expect("the contract serializes");
    let recovered = DataContract::versioned_deserialize_untrusted(&bytes, false, platform_version)
        .expect("the contract deserializes");

    assert_eq!(original, recovered);
    let rules = |contract: &DataContract| {
        contract
            .document_type_for_name("order")
            .expect("the order type")
            .property_constraints()
            .clone()
    };
    assert_eq!(rules(&recovered), rules(&original));
    assert_eq!(rules(&recovered).len(), 1);
}

/// Every stored document was judged against the rules, so none may be added,
/// removed or changed by an update.
#[test]
fn should_refuse_adding_removing_or_changing_rules_on_update() {
    let platform_version = PlatformVersion::latest();
    let rules = json!({ "depositCoversOrder": deposit_rule() });
    let changed = json!({
        "depositCoversOrder": {
            "lessThanOrEqual": [{ "multiply": ["price", "quantity"] }, "deposit"]
        }
    });

    for (before, after) in [
        (None, Some(rules.clone())),
        (Some(rules.clone()), None),
        (Some(rules.clone()), Some(changed)),
    ] {
        let old = order_contract(before.clone(), 1);
        let new = order_contract(after.clone(), 2);
        let result = old
            .validate_update(&new, &BlockInfo::default(), platform_version)
            .expect("the update is judged");
        assert!(
            result.errors.iter().any(|error| matches!(
                error,
                ConsensusError::BasicError(BasicError::IncompatibleDocumentTypeSchemaError(e))
                    if e.document_type_name() == "order"
                        && e.property_path().starts_with("/propertyConstraints")
            )),
            "{before:?} -> {after:?}: {:?}",
            result.errors
        );
    }

    let old = order_contract(Some(rules.clone()), 1);
    let new = order_contract(Some(rules), 2);
    let result = old
        .validate_update(&new, &BlockInfo::default(), platform_version)
        .expect("the update is judged");
    assert!(result.is_valid(), "{:?}", result.errors);
}
