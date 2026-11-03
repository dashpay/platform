//! The `propertyConstraints` doctype keyword under generation 3 (protocol
//! version 14): the rules are parsed onto the document type on both paths,
//! every property a rule reads must be a stored integer property, the limits
//! hold under full validation only, the meta-schema checks the grammar when a
//! contract registers, a contract round-trips through platform serialization
//! with its rules, and any change to them is an incompatible update. Protocol
//! version 13 refuses the keyword when registering and ignores it when reading.

use super::immutable_tests::{
    expect_structure_error, parse_dispatched, parse_dispatched_with_defs,
};
use super::*;
use crate::block::block_info::BlockInfo;
use crate::consensus::basic::basic_error::BasicError;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::document_type::accessors::DocumentTypeV2Getters;
use crate::data_contract::document_type::property_constraints::{
    DocumentSystemValues, ElementKind, PropertyConstraint, PropertyRead, SystemProperty,
};
use crate::data_contract::methods::validate_update::DataContractUpdateValidationMethodsV0;
use crate::data_contract::DataContract;
use crate::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use crate::document::{Document, DocumentV0, DocumentV0Getters};
use crate::serialization::{
    PlatformDeserializableWithPotentialValidationFromVersionedStructureUntrusted,
    PlatformSerializableWithPlatformVersion,
};
use platform_value::platform_value;
use platform_value::string_encoding::Encoding;
use serde_json::json;

/// An `order` type: four required integers, an optional nested `meta` object
/// with an integer `total`, a string, a number, a typed array and an integer
/// `code` to be refused as operands or listed as transient, a boolean `rush`,
/// a string `state` with an `enum` and two identifiers, `buyerId` and
/// `sellerId`.
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
            },
            "rush": { "type": "boolean", "position": 9 },
            "state": {
                "type": "string",
                "enum": ["open", "closed"],
                "maxLength": 10,
                "position": 10
            },
            "buyerId": {
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "maxItems": 32,
                "contentMediaType": "application/x.dash.dpp.identifier",
                "position": 11
            },
            "sellerId": {
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "maxItems": 32,
                "contentMediaType": "application/x.dash.dpp.identifier",
                "position": 12
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

/// `anyOf`, `allOf` and `not` register and parse on both paths, and every property
/// a condition reads, however deep, is held to the same checks as a comparison's.
#[test]
fn should_parse_combined_conditions_and_check_every_property_they_read() {
    let rules = json!({
        "feeWaivedOrAtLeastTen": {
            "anyOf": [{ "equal": ["fee", 0] }, { "greaterThanOrEqual": ["fee", 10] }]
        },
        "noFreeLargeOrder": {
            "not": { "allOf": [{ "equal": ["price", 0] }, { "greaterThan": ["quantity", 10] }] }
        },
        "depositOrSmallOrder": {
            "allOf": [
                {
                    "anyOf": [
                        { "greaterThan": ["deposit", 0] },
                        { "not": { "greaterThan": ["quantity", 1] } }
                    ]
                },
                { "lessThanOrEqual": [{ "ifAbsent": ["meta.total", 0] }, "deposit"] }
            ]
        }
    });
    for full_validation in [true, false] {
        let document_type = parse_order(rules.clone(), full_validation)
            .unwrap_or_else(|e| panic!("full_validation {full_validation}: should parse: {e}"));
        let constraints = document_type.property_constraints();
        assert_eq!(
            constraints["feeWaivedOrAtLeastTen"].property_paths(),
            ["fee", "fee"]
        );
        assert_eq!(
            constraints["noFreeLargeOrder"].property_paths(),
            ["price", "quantity"]
        );
        assert_eq!(
            constraints["depositOrSmallOrder"].property_paths(),
            ["deposit", "quantity", "meta.total", "deposit"]
        );
    }

    let nested_string = json!({
        "rule": {
            "anyOf": [
                { "equal": ["fee", 0] },
                { "not": { "lessThan": [{ "add": ["price", "note"] }, 10] } }
            ]
        }
    });
    for full_validation in [true, false] {
        expect_structure_error(
            parse_order(nested_string.clone(), full_validation),
            "rule \"rule\" reads \"note\", which has type string, not integer or boolean",
        );
    }
}

/// An `in` registers on both paths, and its operand reads by value, so it must
/// read integer properties.
#[test]
fn should_parse_an_in_and_hold_its_operand_to_integer_properties() {
    let rules = json!({
        "rule": { "in": [{ "add": ["fee", { "ifAbsent": ["meta.total", 0] }] }, [0, 10, 25]] }
    });
    for full_validation in [true, false] {
        let document_type = parse_order(rules.clone(), full_validation)
            .unwrap_or_else(|e| panic!("full_validation {full_validation}: should parse: {e}"));
        assert_eq!(
            document_type.property_constraints()["rule"].property_paths(),
            ["fee", "meta.total"]
        );
        expect_structure_error(
            parse_order(
                json!({ "rule": { "in": ["note", [1, 2]] } }),
                full_validation,
            ),
            "rule \"rule\" reads \"note\", which has type string, not integer or boolean",
        );
    }
}

/// An operand may read a boolean property, as 1 for true and 0 for false, on
/// both paths, in a comparison and in an `in`.
#[test]
fn should_let_an_operand_read_a_boolean_property() {
    let rules = json!({
        "rushCostsMore": {
            "greaterThanOrEqual": ["fee", { "multiply": ["rush", 50] }]
        },
        "rushIsFlag": { "in": [{ "ifAbsent": ["rush", 0] }, [0, 1]] }
    });
    for full_validation in [true, false] {
        let document_type = parse_order(rules.clone(), full_validation)
            .unwrap_or_else(|e| panic!("full_validation {full_validation}: should parse: {e}"));
        let constraints = document_type.property_constraints();
        assert_eq!(
            constraints["rushCostsMore"].property_paths(),
            ["fee", "rush"]
        );
        assert_eq!(constraints["rushIsFlag"].property_paths(), ["rush"]);
    }
}

/// A string property is compared with `const` strings by `equal` and
/// `notEqual`, or with the strings an `in` lists, on both paths; a nested one by
/// its dotted path, one without an `enum` with any string.
#[test]
fn should_compare_a_string_property_with_constants() {
    let rules = json!({
        "closedHasTotal": {
            "anyOf": [
                { "notEqual": ["state", { "const": "closed" }] },
                { "present": "meta.total" }
            ]
        },
        "knownNote": { "in": ["note", ["a", "b"]] },
        "tagged": { "equal": [{ "const": "x" }, "meta.tag"] }
    });
    for full_validation in [true, false] {
        let document_type = parse_order(rules.clone(), full_validation)
            .unwrap_or_else(|e| panic!("full_validation {full_validation}: should parse: {e}"));
        let constraints = document_type.property_constraints();
        assert_eq!(
            constraints["closedHasTotal"].property_reads(),
            [
                ("state", PropertyRead::Text),
                ("meta.total", PropertyRead::Presence)
            ]
        );
        assert_eq!(constraints["knownNote"].property_paths(), ["note"]);
        assert_eq!(constraints["tagged"].property_paths(), ["meta.tag"]);
    }
}

/// A constant compared with a property that declares an `enum` must be one of
/// its values, or the property could never hold it; the property must be a
/// string, and not a transient one. On both paths.
#[test]
fn should_hold_string_comparisons_to_string_properties_and_their_enums() {
    for (rules, needle) in [
        (
            json!({ "rule": { "equal": ["state", { "const": "closd" }] } }),
            "rule \"rule\" compares \"state\" with \"closd\", which is not one of its enum values",
        ),
        (
            json!({ "rule": { "in": ["state", ["open", "shut"]] } }),
            "rule \"rule\" compares \"state\" with \"shut\", which is not one of its enum values",
        ),
        (
            json!({ "rule": { "equal": ["price", { "const": "x" }] } }),
            "rule \"rule\" compares \"price\" with a string, but it has type",
        ),
        (
            json!({ "rule": { "in": ["rush", ["yes", "no"]] } }),
            "rule \"rule\" compares \"rush\" with a string, but it has type boolean, not string",
        ),
        (
            json!({ "rule": { "notEqual": ["missing", { "const": "x" }] } }),
            "rule \"rule\" compares \"missing\" with a string, but it is not a string property",
        ),
        (
            json!({ "rule": { "equal": ["meta", { "const": "x" }] } }),
            "rule \"rule\" compares \"meta\" with a string, but it is not a string property",
        ),
        // A string read as a number points at the string forms
        (
            json!({ "rule": { "equal": ["state", "price"] } }),
            "rule \"rule\" reads \"state\", which has type string, not integer or boolean: a \
             string property is compared, by equal or notEqual, with a { \"const\": ... } or \
             another string property",
        ),
        (
            json!({ "rule": { "lessThan": ["state", { "const": "open" }] } }),
            "rule \"rule\" at lessThan compares strings, which only equal and notEqual do",
        ),
    ] {
        for full_validation in [true, false] {
            expect_structure_error(parse_order(rules.clone(), full_validation), needle);
        }
    }

    let schema = order_schema(
        Some(json!({ "rule": { "equal": ["note", { "const": "x" }] } })),
        Some("note"),
    );
    for full_validation in [true, false] {
        expect_structure_error(
            parse_dispatched(
                schema_value(schema.clone()),
                PlatformVersion::latest(),
                full_validation,
            ),
            "rule \"rule\" compares \"note\", which is transient or inside a transient object",
        );
    }
}

/// A string property whose schema is a `$ref` to one of the contract's `$defs`
/// is held to the definition's `enum`, as one declared inline is: a constant, a
/// listed string, a prefix, a default and an element looked for that the enum
/// admits register, and a misspelt one is refused. On both paths.
#[test]
fn should_hold_string_constants_to_the_enum_of_a_referenced_definition() {
    let schema_defs = BTreeMap::from([
        (
            "state".to_string(),
            schema_value(json!({
                "type": "string",
                "enum": ["open", "closed"],
                "maxLength": 10
            })),
        ),
        (
            "labels".to_string(),
            schema_value(json!({
                "type": "array",
                "maxItems": 5,
                "items": { "type": "string", "maxLength": 10, "enum": ["sale", "new"] }
            })),
        ),
    ]);
    let referencing = |rules: serde_json::Value| {
        let mut schema = order_schema(Some(rules), None);
        schema["properties"]["state"] = json!({ "$ref": "#/$defs/state", "position": 10 });
        schema["properties"]["labels"] = json!({ "$ref": "#/$defs/labels", "position": 13 });
        schema_value(schema)
    };
    let parse = |rules: serde_json::Value, full_validation: bool| {
        parse_dispatched_with_defs(
            referencing(rules),
            Some(&schema_defs),
            PlatformVersion::latest(),
            full_validation,
        )
    };

    let rules = json!({
        "closedState": { "equal": ["state", { "const": "closed" }] },
        "listedState": { "in": ["state", ["open", "closed"]] },
        "closingState": { "startsWith": ["state", { "const": "clo" }] },
        "stateDefaultsOpen": {
            "equal": [{ "ifAbsent": ["state", "open"] }, { "const": "open" }]
        },
        "onSale": { "contains": ["labels", { "const": "sale" }] }
    });
    for full_validation in [true, false] {
        let document_type = parse(rules.clone(), full_validation)
            .unwrap_or_else(|e| panic!("full_validation {full_validation}: should parse: {e}"));
        assert_eq!(document_type.property_constraints().len(), 5);
    }

    for (rules, needle) in [
        (
            json!({ "rule": { "equal": ["state", { "const": "closd" }] } }),
            "rule \"rule\" compares \"state\" with \"closd\", which is not one of its enum values",
        ),
        (
            json!({ "rule": { "in": ["state", ["open", "shut"]] } }),
            "rule \"rule\" compares \"state\" with \"shut\", which is not one of its enum values",
        ),
        (
            json!({ "rule": { "endsWith": ["state", { "const": "ed!" }] } }),
            "rule \"rule\" tests whether \"state\" ends with \"ed!\", which none of its enum \
             values does",
        ),
        (
            json!({
                "rule": { "equal": [{ "ifAbsent": ["state", "opne"] }, { "const": "open" }] }
            }),
            "rule \"rule\" gives \"state\" the default \"opne\", which is not one of its enum \
             values",
        ),
        (
            json!({ "rule": { "contains": ["labels", { "const": "old" }] } }),
            "rule \"rule\" compares \"labels\" with \"old\", which is not one of its enum values",
        ),
    ] {
        for full_validation in [true, false] {
            expect_structure_error(parse(rules.clone(), full_validation), needle);
        }
    }
}

/// A `$ref` may name a schema below one of the contract's `$defs`, as
/// `#/$defs/wrapper/properties/inner` does, and the property is then held to
/// the `enum` found there, as the core parse reads it: a constant the enum
/// lists registers, and one it does not is refused. On both paths.
#[test]
fn should_hold_string_constants_to_the_enum_below_a_referenced_definition() {
    let schema_defs = BTreeMap::from([(
        "wrapper".to_string(),
        schema_value(json!({
            "type": "object",
            "properties": {
                "inner": {
                    "type": "string",
                    "enum": ["open", "closed"],
                    "maxLength": 10,
                    "position": 0
                }
            },
            "additionalProperties": false
        })),
    )]);
    let parse = |rules: serde_json::Value, full_validation: bool| {
        let mut schema = order_schema(Some(rules), None);
        schema["properties"]["state"] =
            json!({ "$ref": "#/$defs/wrapper/properties/inner", "position": 10 });
        parse_dispatched_with_defs(
            schema_value(schema),
            Some(&schema_defs),
            PlatformVersion::latest(),
            full_validation,
        )
    };

    for full_validation in [true, false] {
        let document_type = parse(
            json!({ "closedState": { "equal": ["state", { "const": "closed" }] } }),
            full_validation,
        )
        .unwrap_or_else(|e| panic!("full_validation {full_validation}: should parse: {e}"));
        assert_eq!(document_type.property_constraints().len(), 1);

        expect_structure_error(
            parse(
                json!({ "rule": { "equal": ["state", { "const": "closd" }] } }),
                full_validation,
            ),
            "rule \"rule\" compares \"state\" with \"closd\", which is not one of its enum values",
        );
    }
}

/// Two bare paths naming string properties compare the strings, by `equal` or
/// `notEqual` only, on both paths; one string and one integer property stay an
/// integer comparison, refused for its string.
#[test]
fn should_compare_two_string_properties() {
    let rules = json!({
        "noteIsNotTag": { "notEqual": ["note", "meta.tag"] },
        "stateIsNote": { "equal": ["state", "note"] }
    });
    for full_validation in [true, false] {
        let document_type = parse_order(rules.clone(), full_validation)
            .unwrap_or_else(|e| panic!("full_validation {full_validation}: should parse: {e}"));
        let constraints = document_type.property_constraints();
        assert_eq!(
            constraints["noteIsNotTag"].property_reads(),
            [
                ("note", PropertyRead::Text),
                ("meta.tag", PropertyRead::Text)
            ]
        );
        assert_eq!(
            constraints["stateIsNote"].property_reads(),
            [("state", PropertyRead::Text), ("note", PropertyRead::Text)]
        );

        expect_structure_error(
            parse_order(
                json!({ "rule": { "lessThan": ["note", "state"] } }),
                full_validation,
            ),
            "rule \"rule\" at lessThan compares strings, which only equal and notEqual do",
        );
        expect_structure_error(
            parse_order(
                json!({ "rule": { "equal": ["note", "price"] } }),
                full_validation,
            ),
            "rule \"rule\" reads \"note\", which has type string, not integer or boolean",
        );
    }

    let schema = order_schema(
        Some(json!({ "rule": { "notEqual": ["state", "note"] } })),
        Some("note"),
    );
    for full_validation in [true, false] {
        expect_structure_error(
            parse_dispatched(
                schema_value(schema.clone()),
                PlatformVersion::latest(),
                full_validation,
            ),
            "rule \"rule\" compares \"note\", which is transient or inside a transient object",
        );
    }
}

/// An `ifAbsent` with a string default reads a string property on both paths,
/// in `equal`, `notEqual` and `in`; the default, like a constant, must be one
/// of the property's `enum` values, and the property a string.
#[test]
fn should_give_a_string_property_a_default() {
    let rules = json!({
        "stateDefaultsOpen": {
            "equal": [{ "ifAbsent": ["state", "open"] }, { "const": "open" }]
        },
        "noteListed": { "in": [{ "ifAbsent": ["note", "x"] }, ["x", "y"]] },
        "tagIsNotNote": { "notEqual": [{ "ifAbsent": ["meta.tag", "t"] }, "note"] }
    });
    for full_validation in [true, false] {
        let document_type = parse_order(rules.clone(), full_validation)
            .unwrap_or_else(|e| panic!("full_validation {full_validation}: should parse: {e}"));
        let constraints = document_type.property_constraints();
        assert_eq!(
            constraints["stateDefaultsOpen"].text_defaults(),
            [("state", "open")]
        );
        assert_eq!(
            constraints["tagIsNotNote"].property_reads(),
            [
                ("meta.tag", PropertyRead::Text),
                ("note", PropertyRead::Text)
            ]
        );
        assert_eq!(constraints["noteListed"].property_paths(), ["note"]);
    }

    for (rules, needle) in [
        (
            json!({
                "rule": { "equal": [{ "ifAbsent": ["state", "opne"] }, { "const": "open" }] }
            }),
            "rule \"rule\" gives \"state\" the default \"opne\", which is not one of its enum \
             values",
        ),
        (
            json!({ "rule": { "equal": [{ "ifAbsent": ["price", "x"] }, { "const": "x" }] } }),
            "rule \"rule\" compares \"price\" with a string, but it has type",
        ),
        (
            json!({ "rule": { "equal": ["price", { "ifAbsent": ["note", "x"] }] } }),
            "rule \"rule\" compares \"price\" with a string, but it has type",
        ),
    ] {
        for full_validation in [true, false] {
            expect_structure_error(parse_order(rules.clone(), full_validation), needle);
        }
    }
}

/// Identifier properties compare with base58 `const`s, with each other and
/// with the identifiers an `in` lists, on both paths, by `equal` and
/// `notEqual` only; an identifier read as a number, or compared with a string
/// property, is refused.
#[test]
fn should_compare_identifier_properties() {
    let token = Identifier::new([7; 32]).to_string(Encoding::Base58);
    let other = Identifier::new([8; 32]).to_string(Encoding::Base58);
    let rules = json!({
        "boughtWithToken": { "equal": ["buyerId", { "const": token.clone() }] },
        "buyerIsNotSeller": { "notEqual": ["buyerId", "sellerId"] },
        "knownSeller": { "in": ["sellerId", [token.clone(), other.clone()]] }
    });
    for full_validation in [true, false] {
        let document_type = parse_order(rules.clone(), full_validation)
            .unwrap_or_else(|e| panic!("full_validation {full_validation}: should parse: {e}"));
        let constraints = document_type.property_constraints();
        assert_eq!(
            constraints["buyerIsNotSeller"].property_reads(),
            [
                ("buyerId", PropertyRead::Identifier),
                ("sellerId", PropertyRead::Identifier)
            ]
        );
        assert_eq!(constraints["boughtWithToken"].property_paths(), ["buyerId"]);
        assert_eq!(constraints["knownSeller"].property_paths(), ["sellerId"]);
    }

    for (rules, needle) in [
        (
            json!({ "rule": { "lessThan": ["buyerId", "sellerId"] } }),
            "rule \"rule\" at lessThan compares identifiers, which only equal and notEqual do",
        ),
        (
            json!({ "rule": { "equal": ["note", "buyerId"] } }),
            "rule \"rule\" at equal compares a string property with an identifier property",
        ),
        (
            json!({ "rule": { "equal": ["buyerId", "price"] } }),
            "rule \"rule\" reads \"buyerId\", which has type identifier, not integer or boolean: \
             an identifier property is compared",
        ),
        (
            json!({ "rule": { "equal": ["buyerId", { "const": "closed" }] } }),
            "rule \"rule\" at equal[1].const holds \"closed\", which is not a base58 identifier",
        ),
    ] {
        for full_validation in [true, false] {
            expect_structure_error(parse_order(rules.clone(), full_validation), needle);
        }
    }

    let schema = order_schema(
        Some(json!({ "rule": { "notEqual": ["buyerId", "sellerId"] } })),
        Some("buyerId"),
    );
    for full_validation in [true, false] {
        expect_structure_error(
            parse_dispatched(
                schema_value(schema.clone()),
                PlatformVersion::latest(),
                full_validation,
            ),
            "rule \"rule\" compares \"buyerId\", which is transient or inside a transient object",
        );
    }
}

/// `$ownerId` compares as an identifier on both paths, and reads no property;
/// an indexOnly type refuses a rule reading it, since its deletes carry no
/// owner to judge the rule with.
#[test]
fn should_compare_the_owner_and_refuse_it_on_an_index_only_type() {
    let writer = Identifier::new([7; 32]).to_string(Encoding::Base58);
    let other = Identifier::new([8; 32]).to_string(Encoding::Base58);
    let rules = json!({
        "buyerOwns": { "equal": ["buyerId", "$ownerId"] },
        "knownWriter": { "in": ["$ownerId", [writer, other]] },
        "sellerIsNotOwner": { "notEqual": ["sellerId", "$ownerId"] }
    });
    for full_validation in [true, false] {
        let document_type = parse_order(rules.clone(), full_validation)
            .unwrap_or_else(|e| panic!("full_validation {full_validation}: should parse: {e}"));
        let constraints = document_type.property_constraints();
        assert_eq!(constraints["buyerOwns"].property_paths(), ["buyerId"]);
        assert!(constraints["knownWriter"].property_paths().is_empty());
        assert!(constraints.values().all(PropertyConstraint::reads_owner));
    }

    let index_only = |rule: serde_json::Value| {
        schema_value(json!({
            "type": "object",
            "indexOnly": true,
            "documentsMutable": false,
            "indices": [{
                "name": "byTopic",
                "properties": [{ "topic": "asc" }, { "authorId": "asc" }]
            }],
            "properties": {
                "topic": { "type": "string", "maxLength": 50, "position": 0 },
                "authorId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "position": 1
                }
            },
            "required": ["topic", "authorId"],
            "additionalProperties": false,
            "propertyConstraints": { "rule": rule }
        }))
    };
    for full_validation in [true, false] {
        expect_structure_error(
            parse_dispatched(
                index_only(json!({ "equal": ["authorId", "$ownerId"] })),
                PlatformVersion::latest(),
                full_validation,
            ),
            "rule \"rule\" compares $ownerId, which a delete of an indexOnly document does not \
             carry",
        );
        parse_dispatched(
            index_only(json!({
                "notEqual": ["topic", { "const": "x" }]
            })),
            PlatformVersion::latest(),
            full_validation,
        )
        .unwrap_or_else(|e| panic!("a rule not reading the owner registers: {e}"));
    }
}

/// An identifier property that declares `refersTo` is an identifier property
/// all the same: on both paths it compares with a const, with another
/// identifier property whether or not that one declares `refersTo`, with
/// `$ownerId` and in an `in`, and the rules judge its value as they judge any
/// identifier's; read as a number, it is refused as any identifier is.
#[test]
fn should_compare_identifier_properties_that_declare_refers_to() {
    let token = Identifier::new([7; 32]);
    let other = Identifier::new([8; 32]);
    let rules = json!({
        "boughtWithToken": { "equal": ["buyerId", { "const": token.to_string(Encoding::Base58) }] },
        "buyerIsNotSeller": { "notEqual": ["buyerId", "sellerId"] },
        "buyerOwns": { "equal": ["buyerId", "$ownerId"] },
        "knownBuyer": {
            "in": [
                "buyerId",
                [token.to_string(Encoding::Base58), other.to_string(Encoding::Base58)]
            ]
        }
    });
    let referring_schema = |rules: serde_json::Value, referring: &[&str]| {
        let mut schema = order_schema(Some(rules), None);
        for path in referring {
            schema["properties"][*path]["refersTo"] = json!({ "type": "identity" });
        }
        schema_value(schema)
    };

    for referring in [&["buyerId"][..], &["sellerId"], &["buyerId", "sellerId"]] {
        for full_validation in [true, false] {
            let document_type = parse_dispatched(
                referring_schema(rules.clone(), referring),
                PlatformVersion::latest(),
                full_validation,
            )
            .unwrap_or_else(|e| {
                panic!("{referring:?}, full_validation {full_validation}: should parse: {e}")
            });
            for path in referring {
                assert!(matches!(
                    document_type.flattened_properties()[*path].property_type,
                    DocumentPropertyType::IdentifierWithReference(_)
                ));
            }
            let constraints = document_type.property_constraints();
            assert_eq!(
                constraints["buyerIsNotSeller"].property_reads(),
                [
                    ("buyerId", PropertyRead::Identifier),
                    ("sellerId", PropertyRead::Identifier)
                ]
            );
            for name in ["boughtWithToken", "buyerOwns", "knownBuyer"] {
                assert_eq!(
                    constraints[name].property_reads(),
                    [("buyerId", PropertyRead::Identifier)],
                    "{name}"
                );
            }
            assert!(constraints["buyerOwns"].reads_owner());

            let order = |buyer: Identifier| {
                platform_value!({
                    "buyerId": buyer,
                    "sellerId": Identifier::new([9; 32]),
                })
            };
            // The system values of an order owned by `owner`, no time or height
            let owned = |owner: Option<Identifier>| DocumentSystemValues {
                owner_id: owner,
                ..Default::default()
            };
            for (name, owner, holds_for_token, holds_for_other) in [
                ("boughtWithToken", None, true, false),
                ("buyerIsNotSeller", None, true, true),
                ("buyerOwns", Some(token), true, false),
                ("knownBuyer", None, true, true),
            ] {
                assert_eq!(
                    constraints[name].holds(&order(token), &owned(owner)),
                    Ok(holds_for_token),
                    "{name}"
                );
                assert_eq!(
                    constraints[name].holds(&order(other), &owned(owner)),
                    Ok(holds_for_other),
                    "{name}"
                );
            }
            // The seller's id as the buyer's: listed nowhere, and equal to the seller
            for name in ["knownBuyer", "buyerIsNotSeller"] {
                assert_eq!(
                    constraints[name].holds(&order(Identifier::new([9; 32])), &owned(None)),
                    Ok(false),
                    "{name}"
                );
            }
        }
    }

    for (rules, needle) in [
        (
            json!({ "rule": { "equal": ["buyerId", "price"] } }),
            "rule \"rule\" reads \"buyerId\", which has type identifier, not integer or boolean: \
             an identifier property is compared",
        ),
        (
            json!({ "rule": { "equal": ["note", "buyerId"] } }),
            "rule \"rule\" at equal compares a string property with an identifier property",
        ),
        (
            json!({ "rule": { "lessThan": ["buyerId", "sellerId"] } }),
            "rule \"rule\" at lessThan compares identifiers, which only equal and notEqual do",
        ),
    ] {
        for full_validation in [true, false] {
            expect_structure_error(
                parse_dispatched(
                    referring_schema(rules.clone(), &["buyerId"]),
                    PlatformVersion::latest(),
                    full_validation,
                ),
                needle,
            );
        }
    }
}

/// `$ownerId` and the system times and heights are no properties of the type:
/// every document has an owner, and a type that records a time holds it on
/// every document. A presence test of one is refused on both paths (the
/// meta-schema admits them as paths, for comparisons and operands), and one of
/// any other system property the meta-schema refuses when registering.
#[test]
fn should_refuse_a_presence_test_of_a_system_property() {
    for full_validation in [true, false] {
        expect_structure_error(
            parse_order(
                json!({ "rule": { "present": "$ownerId" } }),
                full_validation,
            ),
            "tests the presence of \"$ownerId\", which is not a property of the document type",
        );
    }
    // The meta-schema admits a system time or height as a path, for an operand to
    // read, and the parser refuses a presence test of one on both paths
    for full_validation in [true, false] {
        expect_structure_error(
            parse_order(
                json!({ "rule": { "absent": "$createdAt" } }),
                full_validation,
            ),
            "tests the presence of \"$createdAt\", which is not a property of the document type",
        );
    }
    let rules = json!({ "rule": { "present": "$revision" } });
    let registered = parse_order(rules.clone(), true);
    assert!(
        registered.as_ref().is_err_and(is_json_schema_error),
        "the meta-schema should refuse it, got {registered:?}"
    );
    expect_structure_error(
        parse_order(rules, false),
        "tests the presence of \"$revision\", which is not a property of the document type",
    );
}

/// `present` and `absent` test a property of any type, an object included, on
/// both paths; the path must name a property of the type, and not a transient
/// one.
#[test]
fn should_test_the_presence_of_any_property_the_type_declares() {
    for path in [
        "note",
        "ratio",
        "counts",
        "meta",
        "meta.tag",
        "meta.total",
        "price",
    ] {
        let rules = json!({
            "rule": { "anyOf": [{ "present": path }, { "absent": "fee" }] }
        });
        for full_validation in [true, false] {
            let document_type = parse_order(rules.clone(), full_validation).unwrap_or_else(|e| {
                panic!("{path}, full_validation {full_validation}: should parse: {e}")
            });
            assert_eq!(
                document_type.property_constraints()["rule"].property_paths(),
                [path, "fee"]
            );
        }
    }

    for path in ["missing", "meta.missing", "note.length", "price.value"] {
        for full_validation in [true, false] {
            expect_structure_error(
                parse_order(json!({ "rule": { "present": path } }), full_validation),
                &format!(
                    "rule \"rule\" tests the presence of \"{path}\", which is not a property of \
                     the document type"
                ),
            );
        }
    }

    for (transient, path) in [("note", "note"), ("meta", "meta"), ("meta", "meta.tag")] {
        let schema = order_schema(
            Some(json!({ "rule": { "not": { "absent": path } } })),
            Some(transient),
        );
        for full_validation in [true, false] {
            expect_structure_error(
                parse_dispatched(
                    schema_value(schema.clone()),
                    PlatformVersion::latest(),
                    full_validation,
                ),
                &format!(
                    "rule \"rule\" tests the presence of \"{path}\", which is transient or \
                     inside a transient object"
                ),
            );
        }
    }
}

/// Only an integer property's value is a number the rule can compute with: a
/// string, a float, an array, an object and a system property are refused on
/// both paths, as is a path naming nothing.
#[test]
fn should_refuse_a_rule_reading_anything_but_an_integer_property() {
    for (operand, needle) in [
        (
            "note",
            "reads \"note\", which has type string, not integer or boolean",
        ),
        (
            "ratio",
            "reads \"ratio\", which has type f64, not integer or boolean",
        ),
        (
            "counts",
            "reads \"counts\", which has type array, not integer or boolean",
        ),
        (
            "meta.tag",
            "reads \"meta.tag\", which has type string, not integer or boolean",
        ),
        (
            "meta",
            "reads \"meta\", which is not an integer or boolean property of the document type",
        ),
        (
            "missing",
            "reads \"missing\", which is not an integer or boolean property",
        ),
        (
            "meta.missing",
            "reads \"meta.missing\", which is not an integer or boolean property",
        ),
        (
            "$ownerId",
            "reads \"$ownerId\", which is not an integer or boolean property",
        ),
        (
            "$createdAt",
            "reads $createdAt, which the document type does not record: list it in required",
        ),
        (
            "$revision",
            "reads \"$revision\", which is not an integer or boolean property",
        ),
    ] {
        for full_validation in [true, false] {
            let result = parse_order(
                json!({ "rule": { "lessThan": [operand, "price"] } }),
                full_validation,
            );
            // The meta-schema refuses a `$` in a path when registering, but for
            // `$ownerId`, which only a comparison of identifiers may read, and the
            // system times and heights an operand may read
            if full_validation
                && operand.starts_with('$')
                && operand != "$ownerId"
                && SystemProperty::from_name(operand).is_none()
            {
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
    for (transient, operand, nested) in [
        ("code", "code", false),
        ("meta", "meta.total", false),
        ("code", "code", true),
    ] {
        // Also when the property is read deep inside a condition
        let rule = if nested {
            json!({
                "anyOf": [
                    { "equal": ["price", 1] },
                    { "not": { "lessThan": [{ "add": ["fee", operand] }, "price"] } }
                ]
            })
        } else {
            json!({ "lessThan": [operand, "price"] })
        };
        let schema = order_schema(Some(json!({ "rule": rule })), Some(transient));
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

    // Every logical operator and every comparison counts too: allOf, the equal with
    // its add, "price", ones and 0, and not over an equal of "fee" and 0
    let logical_rule_of = |nodes: usize| {
        let mut operands = vec![json!("price")];
        operands.resize(nodes - 8, json!(1));
        json!({
            "rule": {
                "allOf": [
                    { "equal": [{ "add": operands }, 0] },
                    { "not": { "equal": ["fee", 0] } }
                ]
            }
        })
    };
    let document_type = parse_order(logical_rule_of(max_nodes), true)
        .expect("the most nodes a rule may have, logical ones included");
    assert_eq!(
        document_type.property_constraints()["rule"].node_count(),
        max_nodes
    );
    expect_structure_error(
        parse_order(logical_rule_of(max_nodes + 1), true),
        &format!(
            "rule \"rule\" has {} nodes, above the maximum of {max_nodes}",
            max_nodes + 1
        ),
    );
    parse_order(logical_rule_of(max_nodes + 1), false).expect("a stored contract stays readable");

    // An in is one node, its operand one more, and each value it lists one
    let in_rule_of = |nodes: usize| {
        let values: Vec<_> = (0..nodes - 2).map(|value| json!(value)).collect();
        json!({ "rule": { "in": ["price", values] } })
    };
    let document_type =
        parse_order(in_rule_of(max_nodes), true).expect("the most values an in may list");
    assert_eq!(
        document_type.property_constraints()["rule"].node_count(),
        max_nodes
    );
    expect_structure_error(
        parse_order(in_rule_of(max_nodes + 1), true),
        &format!(
            "rule \"rule\" has {} nodes, above the maximum of {max_nodes}",
            max_nodes + 1
        ),
    );
    parse_order(in_rule_of(max_nodes + 1), false).expect("a stored contract stays readable");
}

/// No `anyOf` or `allOf` may list the same condition twice, checked when a contract
/// registers: the meta-schema refuses two identical JSON conditions, and the parser
/// two that parse alike. A stored contract stays readable.
#[test]
fn should_refuse_a_repeated_condition_under_full_validation_only() {
    let identical = json!({
        "rule": { "anyOf": [{ "equal": ["price", 1] }, { "equal": ["price", 1] }] }
    });
    let registered = parse_order(identical.clone(), true);
    assert!(
        registered.as_ref().is_err_and(is_json_schema_error),
        "the meta-schema should refuse it, got {registered:?}"
    );
    parse_order(identical, false).expect("a stored contract stays readable");

    // A path on its own reads as ifAbsent 0, so these two are the same condition
    let alike = json!({
        "rule": {
            "allOf": [
                { "equal": ["fee", 1] },
                {
                    "not": {
                        "anyOf": [
                            { "equal": ["price", 1] },
                            { "equal": [{ "ifAbsent": ["price", 0] }, 1] }
                        ]
                    }
                }
            ]
        }
    });
    expect_structure_error(
        parse_order(alike.clone(), true),
        "rule \"rule\" at allOf[1].not.anyOf[1] repeats the condition at allOf[1].not.anyOf[0]",
    );
    parse_order(alike, false).expect("a stored contract stays readable");
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
        json!({ "rule": { "equal": [{ "ifAbsent": ["price", true] }, 1] } }),
        json!({ "bad-name": { "equal": ["price", 1] } }),
        json!(["price"]),
        json!({ "rule": { "or": [{ "equal": ["price", 1] }, { "equal": ["fee", 1] }] } }),
        json!({ "rule": { "anyOf": [{ "equal": ["price", 1] }] } }),
        json!({ "rule": { "allOf": { "equal": ["price", 1] } } }),
        json!({ "rule": { "not": [{ "equal": ["price", 1] }] } }),
        json!({ "rule": { "not": { "equal": ["price", 1] }, "equal": ["fee", 1] } }),
        json!({
            "rule": {
                "anyOf": [
                    { "anyOf": [{ "equal": ["price", 1] }, { "equal": ["price", 2] }] },
                    { "equal": ["fee", 1] }
                ]
            }
        }),
        json!({
            "rule": {
                "allOf": [
                    { "equal": ["fee", 1] },
                    { "allOf": [{ "equal": ["price", 1] }, { "equal": ["price", 2] }] }
                ]
            }
        }),
        json!({ "rule": { "not": { "not": { "equal": ["price", 1] } } } }),
        json!({ "rule": { "not": { "notIn": ["price", [1, 2]] } } }),
        json!({ "rule": { "anyOf": [{ "equal": ["price", 1] }, { "equal": ["price"] }] } }),
        json!({ "rule": { "present": 1 } }),
        json!({ "rule": { "absent": ["note"] } }),
        json!({ "rule": { "present": "note", "absent": "fee" } }),
        json!({ "rule": { "not": { "present": { "add": ["price", 1] } } } }),
        json!({ "rule": { "in": ["price"] } }),
        json!({ "rule": { "in": ["price", [1]] } }),
        json!({ "rule": { "in": ["price", [1, 1]] } }),
        json!({ "rule": { "in": ["price", [1, 1.5]] } }),
        json!({ "rule": { "in": ["price", [1, "fee"]] } }),
        json!({ "rule": { "in": ["price", [1, 2], 3] } }),
        json!({ "rule": { "in": ["price", 1] } }),
        json!({ "rule": { "equal": ["state", { "const": 5 }] } }),
        json!({ "rule": { "in": ["state", ["open", 2]] } }),
        json!({ "rule": { "in": ["state", ["open"]] } }),
        json!({ "rule": { "in": ["state", ["open", "open"]] } }),
        json!({ "rule": { "lessThan": [{ "length": 5 }, 10] } }),
        json!({ "rule": { "lessThan": [{ "count": ["counts"] }, 10] } }),
        json!({ "rule": { "lessThan": [{ "size": "note" }, 10] } }),
        json!({ "rule": { "lessThan": [{ "length": "note", "count": "counts" }, 10] } }),
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
        (
            json!({ "rule": { "anyOf": [{ "equal": ["price", 1] }, { "equal": [1, 1] }] } }),
            "rule \"rule\" at anyOf[1] reads no property",
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
    // Typed arrays arrived with protocol version 14 as well; the property after
    // them takes their position, so the positions stay contiguous
    schema["properties"]
        .as_object_mut()
        .expect("the properties")
        .remove("counts");
    schema["properties"]["rush"]["position"] = json!(8);
    schema["properties"]["state"]["position"] = json!(9);
    schema["properties"]["buyerId"]["position"] = json!(10);
    schema["properties"]["sellerId"]["position"] = json!(11);
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
    contract_with_order_type(order_schema(rules, None), version)
}

/// A contract whose `order` type is declared by `schema`.
fn contract_with_order_type(schema: serde_json::Value, version: u32) -> DataContract {
    let contract = json!({
        "$formatVersion": "1",
        "id": Identifier::from(CONTRACT_ID).to_string(Encoding::Base58),
        "ownerId": Identifier::from([8; 32]).to_string(Encoding::Base58),
        "version": version,
        "documentSchemas": { "order": schema }
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

/// A stored document keeps no object none of whose members it holds: `{}`,
/// and `{ "inner": {} }` around one, are read back as no object at all. So
/// `present` and `absent` judge such an object absent in the data a create
/// carries too, and every rule reaches the same verdict on a document's data
/// as on the document read back from storage, which a transfer, a purchase
/// and a price update are judged on.
#[test]
fn should_judge_presence_alike_on_the_data_and_on_the_stored_document() {
    let platform_version = PlatformVersion::latest();
    let mut schema = order_schema(
        Some(json!({
            "metaOrSeller": {
                "anyOf": [{ "present": "meta" }, { "equal": ["sellerId", "$ownerId"] }]
            },
            "noMetaOrSeller": {
                "anyOf": [{ "absent": "meta" }, { "equal": ["sellerId", "$ownerId"] }]
            },
            "noInner": { "absent": "meta.inner" }
        })),
        None,
    );
    schema["properties"]["meta"]["properties"]["inner"] = json!({
        "type": "object",
        "position": 2,
        "properties": { "note": { "type": "string", "maxLength": 30, "position": 0 } },
        "additionalProperties": false
    });
    let contract = contract_with_order_type(schema, 1);
    let order_type = contract
        .document_type_for_name("order")
        .expect("the order type");

    for (meta, kept) in [
        (platform_value!({}), false),
        (platform_value!({ "inner": {} }), false),
        (platform_value!({ "tag": "x" }), true),
    ] {
        // Owned by someone other than its seller, so `meta` alone decides
        let document: Document = DocumentV0 {
            id: Identifier::new([5; 32]),
            owner_id: Identifier::new([3; 32]),
            properties: BTreeMap::from([
                ("price".to_string(), Value::U64(100)),
                ("fee".to_string(), Value::U64(10)),
                ("quantity".to_string(), Value::U64(2)),
                ("deposit".to_string(), Value::U64(220)),
                ("sellerId".to_string(), Value::Identifier([4; 32])),
                ("meta".to_string(), meta.clone()),
            ]),
            revision: Some(1),
            ..Default::default()
        }
        .into();
        let bytes = document
            .serialize(order_type, &contract, platform_version)
            .expect("the document serializes");
        let stored = Document::from_bytes(&bytes, order_type, platform_version)
            .expect("the document deserializes");
        assert_eq!(
            stored.properties().contains_key("meta"),
            kept,
            "{meta:?}: the stored document keeps meta"
        );

        let system = DocumentSystemValues::of_document(&document);
        let data = Value::from(document.properties().clone());
        let stored_data = Value::from(stored.properties().clone());
        let rules = order_type.property_constraints();
        for (name, rule) in rules {
            assert_eq!(
                rule.violation(&data, &system),
                rule.violation(&stored_data, &system),
                "{name} on {meta:?}"
            );
        }
        assert_eq!(
            rules["metaOrSeller"].violation(&data, &system).is_none(),
            kept,
            "{meta:?}"
        );
        assert_eq!(
            rules["noMetaOrSeller"].violation(&data, &system).is_none(),
            !kept,
            "{meta:?}"
        );
        assert_eq!(rules["noInner"].violation(&data, &system), None, "{meta:?}");
    }
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

/// `length` and `byteLength` measure a string property and `count` counts the
/// items of an array property, nested ones included, on both paths.
#[test]
fn should_measure_strings_and_count_arrays_on_both_paths() {
    let rules = json!({
        "noteFitsQuantity": {
            "lessThanOrEqual": [{ "length": "note" }, { "multiply": ["quantity", 10] }]
        },
        "tagWithinBytes": { "lessThanOrEqual": [{ "byteLength": "meta.tag" }, 20] },
        "countsPerUnit": { "lessThanOrEqual": [{ "count": "counts" }, "quantity"] }
    });
    for full_validation in [true, false] {
        let document_type = parse_order(rules.clone(), full_validation)
            .unwrap_or_else(|e| panic!("full_validation {full_validation}: should parse: {e}"));
        let constraints = document_type.property_constraints();
        assert_eq!(
            constraints["noteFitsQuantity"].property_reads(),
            [
                ("note", PropertyRead::Length),
                ("quantity", PropertyRead::Value)
            ]
        );
        assert_eq!(
            constraints["tagWithinBytes"].property_reads(),
            [("meta.tag", PropertyRead::Length)]
        );
        assert_eq!(
            constraints["countsPerUnit"].property_reads(),
            [
                ("counts", PropertyRead::Count),
                ("quantity", PropertyRead::Value)
            ]
        );
    }

    // A byte array counts its bytes: here a signature is left out or 64 or 65
    // bytes long
    let schema = platform_value!({
        "type": "object",
        "properties": {
            "signature": {
                "type": "array",
                "byteArray": true,
                "maxItems": 65,
                "position": 0
            }
        },
        "propertyConstraints": {
            "signatureLength": { "in": [{ "count": "signature" }, [0, 64, 65]] }
        },
        "additionalProperties": false
    });
    for full_validation in [true, false] {
        let document_type =
            parse_dispatched(schema.clone(), PlatformVersion::latest(), full_validation)
                .expect("a rule may count the bytes of a byte array");
        assert_eq!(
            document_type.property_constraints()["signatureLength"].property_reads(),
            [("signature", PropertyRead::Count)]
        );
    }
}

/// A size reads a property of the type of its measure, stored: `length` and
/// `byteLength` a string, `count` an array or a byte array.
#[test]
fn should_hold_a_size_to_the_property_it_measures() {
    for (operand, needle) in [
        (
            json!({ "length": "counts" }),
            "measures the length of \"counts\", which has type array, not string: count gives \
             the items of an array or byte array",
        ),
        (
            json!({ "byteLength": "buyerId" }),
            "measures the length of \"buyerId\", which has type identifier, not string",
        ),
        (
            json!({ "length": "meta" }),
            "measures the length of \"meta\", which is not a string property of the document type",
        ),
        (
            json!({ "byteLength": "$ownerId" }),
            "measures the length of \"$ownerId\", which is not a string property",
        ),
        (
            json!({ "count": "note" }),
            "counts the items of \"note\", which has type string, not array: length or \
             byteLength gives the size of a string",
        ),
        (
            json!({ "count": "buyerId" }),
            "counts the items of \"buyerId\", which has type identifier, not array or byteArray",
        ),
        (
            json!({ "count": "rush" }),
            "counts the items of \"rush\", which has type boolean, not array or byteArray",
        ),
        (
            json!({ "count": "meta.missing" }),
            "counts the items of \"meta.missing\", which is not an array or byte array property",
        ),
    ] {
        for full_validation in [true, false] {
            expect_structure_error(
                parse_order(
                    json!({ "rule": { "lessThan": [operand.clone(), "price"] } }),
                    full_validation,
                ),
                needle,
            );
        }
    }

    // A transient value is never stored, so no rule may measure one
    for (transient, operand, path) in [
        ("note", json!({ "length": "note" }), "note"),
        ("meta", json!({ "byteLength": "meta.tag" }), "meta.tag"),
        ("counts", json!({ "count": "counts" }), "counts"),
    ] {
        let verb = if operand.get("count").is_some() {
            "counts the items of"
        } else {
            "measures"
        };
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
                &format!("{verb} \"{path}\", which is transient or inside a transient object"),
            );
        }
    }
}

/// A rule reads a system time or height the type records, by listing it in
/// `required`, on both paths; one the type does not record is refused, and so
/// is any on an indexOnly type, whose deletes carry none.
#[test]
fn should_read_the_system_times_and_heights_the_type_records() {
    let rules = json!({
        "depositAfterCreation": { "greaterThan": ["deposit", "$createdAt"] },
        "pricedAboveHeight": { "lessThan": ["$updatedAtBlockHeight", "price"] },
        "transferredOnCore": { "notEqual": ["$transferredAtCoreBlockHeight", 0] }
    });
    let recording = |required: &[&str]| {
        let mut schema = order_schema(Some(rules.clone()), None);
        let mut names = vec!["price", "fee", "quantity", "deposit"];
        names.extend_from_slice(required);
        schema["required"] = json!(names);
        schema_value(schema)
    };
    for full_validation in [true, false] {
        let document_type = parse_dispatched(
            recording(&[
                "$createdAt",
                "$updatedAtBlockHeight",
                "$transferredAtCoreBlockHeight",
            ]),
            PlatformVersion::latest(),
            full_validation,
        )
        .unwrap_or_else(|e| panic!("full_validation {full_validation}: should parse: {e}"));
        let constraints = document_type.property_constraints();
        assert_eq!(
            constraints["depositAfterCreation"].system_reads(),
            [SystemProperty::CreatedAt]
        );
        assert_eq!(
            constraints["depositAfterCreation"].property_paths(),
            ["deposit"]
        );
        assert_eq!(
            constraints["pricedAboveHeight"].system_reads(),
            [SystemProperty::UpdatedAtBlockHeight]
        );
        assert_eq!(
            constraints["transferredOnCore"].system_reads(),
            [SystemProperty::TransferredAtCoreBlockHeight]
        );

        // Not recording `$updatedAtBlockHeight`, the type may not read it
        expect_structure_error(
            parse_dispatched(
                recording(&["$createdAt", "$transferredAtCoreBlockHeight"]),
                PlatformVersion::latest(),
                full_validation,
            ),
            "rule \"pricedAboveHeight\" reads $updatedAtBlockHeight, which the document type \
             does not record: list it in required",
        );
    }

    let index_only = schema_value(json!({
        "type": "object",
        "indexOnly": true,
        "documentsMutable": false,
        "indices": [{
            "name": "byTopic",
            "properties": [{ "topic": "asc" }, { "until": "asc" }]
        }],
        "properties": {
            "topic": { "type": "string", "maxLength": 50, "position": 0 },
            "until": { "type": "integer", "minimum": 0, "position": 1 }
        },
        "required": ["topic", "until", "$createdAt"],
        "additionalProperties": false,
        "propertyConstraints": { "rule": { "lessThan": ["$createdAt", "until"] } }
    }));
    for full_validation in [true, false] {
        expect_structure_error(
            parse_dispatched(
                index_only.clone(),
                PlatformVersion::latest(),
                full_validation,
            ),
            "rule \"rule\" reads $createdAt, which a delete of an indexOnly document does not \
             carry",
        );
    }

    // The meta-schema admits exactly the nine names
    for name in ["$createdAtHeight", "$deletedAt", "$updatedAtCoreHeight"] {
        let registered = parse_order(json!({ "rule": { "lessThan": [name, "price"] } }), true);
        assert!(
            registered.as_ref().is_err_and(is_json_schema_error),
            "{name}: the meta-schema should refuse it, got {registered:?}"
        );
    }
}

/// A `listing` type with typed arrays of strings (with an `enum`), of
/// identifiers, of integers and of booleans, a byte array, a string, an
/// integer and an identifier, declaring `rules`, `transient` listed transient.
fn listing_schema(rules: serde_json::Value, transient: Option<&str>) -> Value {
    let mut schema = json!({
        "type": "object",
        "properties": {
            "labels": {
                "type": "array",
                "maxItems": 5,
                "items": { "type": "string", "maxLength": 10, "enum": ["sale", "new", "used"] },
                "position": 0
            },
            "members": {
                "type": "array",
                "maxItems": 5,
                "items": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier"
                },
                "position": 1
            },
            "scores": {
                "type": "array",
                "maxItems": 5,
                "items": { "type": "integer", "minimum": 0, "maximum": 100 },
                "position": 2
            },
            "flags": {
                "type": "array",
                "maxItems": 2,
                "items": { "type": "boolean" },
                "position": 3
            },
            "signature": { "type": "array", "byteArray": true, "maxItems": 65, "position": 4 },
            "status": { "type": "string", "maxLength": 10, "position": 5 },
            "pick": { "type": "integer", "minimum": 0, "maximum": 100, "position": 6 },
            "buyerId": {
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "maxItems": 32,
                "contentMediaType": "application/x.dash.dpp.identifier",
                "position": 7
            }
        },
        "additionalProperties": false,
        "propertyConstraints": rules
    });
    if let Some(transient) = transient {
        schema["transient"] = json!([transient]);
    }
    schema_value(schema)
}

/// `contains` looks among the elements of a typed array of integers, strings
/// or identifiers, on both paths, for what the array's elements are.
#[test]
fn should_look_among_the_elements_of_typed_arrays() {
    let rules = json!({
        "onSale": { "contains": ["labels", { "const": "sale" }] },
        "labelledAsStatus": { "contains": ["labels", "status"] },
        "ownerIsMember": { "contains": ["members", "$ownerId"] },
        "buyerIsMember": { "contains": ["members", "buyerId"] },
        "pickScored": { "not": { "contains": ["scores", "pick"] } }
    });
    for full_validation in [true, false] {
        let document_type = parse_dispatched(
            listing_schema(rules.clone(), None),
            PlatformVersion::latest(),
            full_validation,
        )
        .unwrap_or_else(|e| panic!("full_validation {full_validation}: should parse: {e}"));
        let constraints = document_type.property_constraints();
        assert_eq!(
            constraints["onSale"].property_reads(),
            [("labels", PropertyRead::Elements(ElementKind::Text))]
        );
        assert_eq!(
            constraints["labelledAsStatus"].property_reads(),
            [
                ("labels", PropertyRead::Elements(ElementKind::Text)),
                ("status", PropertyRead::Text)
            ]
        );
        assert!(constraints["ownerIsMember"].reads_owner());
        assert_eq!(
            constraints["buyerIsMember"].property_reads(),
            [
                ("members", PropertyRead::Elements(ElementKind::Identifier)),
                ("buyerId", PropertyRead::Identifier)
            ]
        );
        assert_eq!(
            constraints["pickScored"].property_reads(),
            [
                ("scores", PropertyRead::Elements(ElementKind::Integer)),
                ("pick", PropertyRead::Value)
            ]
        );
    }
}

/// The array must be a typed array whose elements are of the kind looked for,
/// stored, and a constant one of the elements' enum values; what is looked
/// for is held to its own kind's checks.
#[test]
fn should_hold_contains_to_arrays_of_the_kind_looked_for() {
    for (rule, needle) in [
        (
            json!({ "contains": ["labels", { "const": "old" }] }),
            "rule \"rule\" compares \"labels\" with \"old\", which is not one of its enum values",
        ),
        (
            json!({ "contains": ["flags", 1] }),
            "rule \"rule\" looks in \"flags\" for an integer, but its elements have type boolean",
        ),
        (
            json!({ "contains": ["status", { "const": "sale" }] }),
            "rule \"rule\" looks in \"status\", which has type string, not an array with items",
        ),
        (
            json!({ "contains": ["signature", 64] }),
            "rule \"rule\" looks in \"signature\", which has type byteArray, not an array with \
             items",
        ),
        (
            json!({ "contains": ["missing", 1] }),
            "rule \"rule\" looks in \"missing\", which is not an array property of the document \
             type",
        ),
        (
            json!({ "contains": ["scores", "status"] }),
            "reads \"status\", which has type string, not integer or boolean",
        ),
        (
            json!({ "contains": ["labels", "buyerId"] }),
            "compares \"buyerId\" with a string, but it has type identifier, not string",
        ),
        (
            json!({ "contains": ["members", "status"] }),
            "compares \"status\" with an identifier, but it has type string, not identifier",
        ),
    ] {
        for full_validation in [true, false] {
            expect_structure_error(
                parse_dispatched(
                    listing_schema(json!({ "rule": rule.clone() }), None),
                    PlatformVersion::latest(),
                    full_validation,
                ),
                needle,
            );
        }
    }

    // A transient array is never stored, so no rule may look in one
    for full_validation in [true, false] {
        expect_structure_error(
            parse_dispatched(
                listing_schema(
                    json!({ "rule": { "contains": ["labels", { "const": "sale" }] } }),
                    Some("labels"),
                ),
                PlatformVersion::latest(),
                full_validation,
            ),
            "looks in \"labels\", which is transient or inside a transient object",
        );
    }

    // The meta-schema checks the shape when registering
    for rules in [
        json!({ "rule": { "contains": ["labels"] } }),
        json!({ "rule": { "contains": ["labels", "status", "pick"] } }),
        json!({ "rule": { "contains": "labels" } }),
    ] {
        let registered = parse_dispatched(
            listing_schema(rules.clone(), None),
            PlatformVersion::latest(),
            true,
        );
        assert!(
            registered.as_ref().is_err_and(is_json_schema_error),
            "{rules}: the meta-schema should refuse it, got {registered:?}"
        );
    }
}

/// `startsWith` and `endsWith` test string properties, on both paths; a
/// constant tested against a property with an `enum` must start or end one of
/// its values; a side that is no stored string property is refused.
#[test]
fn should_test_string_properties_for_prefixes_and_suffixes() {
    let rules = json!({
        "refNote": { "startsWith": ["note", { "const": "ref:" }] },
        "tagSuffix": { "endsWith": ["note", "meta.tag"] },
        "openish": { "startsWith": ["state", { "const": "op" }] },
        "closedish": { "endsWith": ["state", { "const": "sed" }] }
    });
    for full_validation in [true, false] {
        let document_type = parse_order(rules.clone(), full_validation)
            .unwrap_or_else(|e| panic!("full_validation {full_validation}: should parse: {e}"));
        let constraints = document_type.property_constraints();
        assert_eq!(
            constraints["tagSuffix"].property_reads(),
            [
                ("note", PropertyRead::Text),
                ("meta.tag", PropertyRead::Text)
            ]
        );
        assert_eq!(
            constraints["refNote"].property_reads(),
            [("note", PropertyRead::Text)]
        );
    }

    for (rule, needle) in [
        (
            json!({ "startsWith": ["state", { "const": "x" }] }),
            "rule \"rule\" tests whether \"state\" starts with \"x\", which none of its enum \
             values does",
        ),
        (
            json!({ "not": { "endsWith": ["state", { "const": "xyz" }] } }),
            "rule \"rule\" tests whether \"state\" ends with \"xyz\", which none of its enum \
             values does",
        ),
        (
            json!({ "startsWith": ["price", { "const": "1" }] }),
            "compares \"price\" with a string, but it has type",
        ),
        (
            json!({ "endsWith": ["buyerId", { "const": "a" }] }),
            "compares \"buyerId\" with a string, but it has type identifier, not string",
        ),
        (
            json!({ "startsWith": ["note", "missing"] }),
            "compares \"missing\" with a string, but it is not a string property",
        ),
    ] {
        for full_validation in [true, false] {
            expect_structure_error(
                parse_order(json!({ "rule": rule.clone() }), full_validation),
                needle,
            );
        }
    }

    // A transient string is never stored, so no rule may test one
    let schema = order_schema(
        Some(json!({ "rule": { "startsWith": ["note", { "const": "ref:" }] } })),
        Some("note"),
    );
    for full_validation in [true, false] {
        expect_structure_error(
            parse_dispatched(
                schema_value(schema.clone()),
                PlatformVersion::latest(),
                full_validation,
            ),
            "compares \"note\", which is transient or inside a transient object",
        );
    }

    // The meta-schema checks the shape when registering
    for rules in [
        json!({ "rule": { "startsWith": ["note"] } }),
        json!({ "rule": { "endsWith": "note" } }),
        json!({ "rule": { "startsWith": ["note", { "const": "a" }, "meta.tag"] } }),
    ] {
        let registered = parse_order(rules.clone(), true);
        assert!(
            registered.as_ref().is_err_and(is_json_schema_error),
            "{rules}: the meta-schema should refuse it, got {registered:?}"
        );
    }
}

/// `min`, `max`, `abs`, `ifThen`, `ifThenElse` and `notIn` register on both
/// paths, their reads held to the same checks as any; a `notIn` string is
/// checked against the property's enum, and an `ifThen` or `ifThenElse`
/// holding two alike conditions is refused under full validation only, like
/// any repeated condition.
#[test]
fn should_register_min_max_abs_if_then_and_not_in() {
    let seller = Identifier::new([5; 32]).to_string(Encoding::Base58);
    let other = Identifier::new([6; 32]).to_string(Encoding::Base58);
    let rules = json!({
        "feeCapped": { "lessThanOrEqual": ["fee", { "max": [10, { "divide": ["price", 10] }] }] },
        "cheapSide": { "greaterThanOrEqual": [{ "min": ["price", "fee"] }, 1] },
        "depositNearOrder": {
            "lessThanOrEqual": [{ "abs": { "subtract": ["deposit", "price"] } }, 1000]
        },
        "closedHasNote": {
            "ifThen": [{ "equal": ["state", { "const": "closed" }] }, { "present": "note" }]
        },
        "feeByState": {
            "ifThenElse": [
                { "equal": ["state", { "const": "open" }] },
                { "lessThanOrEqual": ["fee", 50] },
                { "lessThanOrEqual": ["fee", 10] }
            ]
        },
        "feeNotBanned": { "notIn": ["fee", [13, 666]] },
        "notSpam": { "notIn": ["note", ["spam", "scam"]] },
        "notTheseSellers": { "notIn": ["sellerId", [seller.clone(), other.clone()]] }
    });
    for full_validation in [true, false] {
        let document_type = parse_order(rules.clone(), full_validation)
            .unwrap_or_else(|e| panic!("full_validation {full_validation}: should parse: {e}"));
        let constraints = document_type.property_constraints();
        assert_eq!(constraints.len(), 8);
        assert_eq!(
            constraints["closedHasNote"].property_reads(),
            [
                ("state", PropertyRead::Text),
                ("note", PropertyRead::Presence)
            ]
        );
        assert_eq!(
            constraints["feeByState"].property_paths(),
            ["state", "fee", "fee"]
        );
        assert_eq!(
            constraints["depositNearOrder"].property_paths(),
            ["deposit", "price"]
        );
    }

    // The reads inside are held to the usual checks
    for (rule, needle) in [
        (
            json!({ "notIn": ["state", ["open", "x"]] }),
            "rule \"rule\" compares \"state\" with \"x\", which is not one of its enum values",
        ),
        (
            json!({ "equal": [{ "abs": "note" }, 1] }),
            "reads \"note\", which has type string, not integer or boolean",
        ),
        (
            json!({ "ifThen": [{ "present": "note" }, { "lessThan": ["missing", 1] }] }),
            "reads \"missing\", which is not an integer or boolean property",
        ),
        (
            json!({
                "ifThenElse": [
                    { "present": "note" },
                    { "present": "fee" },
                    { "equal": ["state", { "const": "x" }] }
                ]
            }),
            "rule \"rule\" compares \"state\" with \"x\", which is not one of its enum values",
        ),
    ] {
        for full_validation in [true, false] {
            expect_structure_error(
                parse_order(json!({ "rule": rule.clone() }), full_validation),
                needle,
            );
        }
    }

    // Two alike conditions say what a simpler rule says
    for (same, needle) in [
        (
            json!({ "ifThen": [{ "present": "note" }, { "present": "note" }] }),
            "rule \"rule\" at ifThen[1] repeats the condition at ifThen[0]",
        ),
        (
            json!({
                "ifThenElse": [{ "present": "note" }, { "present": "fee" }, { "present": "fee" }]
            }),
            "rule \"rule\" at ifThenElse[2] repeats the condition at ifThenElse[1]",
        ),
    ] {
        let same = json!({ "rule": same });
        expect_structure_error(parse_order(same.clone(), true), needle);
        parse_order(same, false).expect("a stored rule is not re-judged for repeats");
    }

    // The meta-schema checks the shapes when registering
    for rules in [
        json!({ "rule": { "ifThen": [{ "present": "note" }] } }),
        json!({ "rule": { "ifThen": { "present": "note" } } }),
        json!({
            "rule": { "ifThen": [{ "present": "note" }, { "present": "fee" }, { "present": "id" }] }
        }),
        json!({ "rule": { "ifThenElse": [{ "present": "note" }, { "present": "fee" }] } }),
        json!({
            "rule": {
                "ifThenElse": [
                    { "present": "note" },
                    { "present": "fee" },
                    { "present": "id" },
                    { "present": "x" }
                ]
            }
        }),
        json!({ "rule": { "notIn": ["price"] } }),
        json!({ "rule": { "notIn": ["price", [1, 1]] } }),
        json!({ "rule": { "equal": [{ "min": ["price"] }, 1] } }),
        json!({ "rule": { "equal": [{ "abs": ["price", "fee"] }, 1] } }),
    ] {
        let registered = parse_order(rules.clone(), true);
        assert!(
            registered.as_ref().is_err_and(is_json_schema_error),
            "{rules}: the meta-schema should refuse it, got {registered:?}"
        );
        expect_structure_error(parse_order(rules, false), "propertyConstraints");
    }
}
