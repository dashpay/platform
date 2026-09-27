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
use crate::data_contract::document_type::property_constraints::{PropertyConstraint, PropertyRead};
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

/// `$ownerId` is the document's owner, which every document has, not a
/// property of the type: a presence test of it is refused on both paths, and
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
    let rules = json!({ "rule": { "present": "$createdAt" } });
    let registered = parse_order(rules.clone(), true);
    assert!(
        registered.as_ref().is_err_and(is_json_schema_error),
        "the meta-schema should refuse it, got {registered:?}"
    );
    expect_structure_error(
        parse_order(rules, false),
        "tests the presence of \"$createdAt\", which is not a property of the document type",
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
            "reads \"$createdAt\", which is not an integer or boolean property",
        ),
    ] {
        for full_validation in [true, false] {
            let result = parse_order(
                json!({ "rule": { "lessThan": [operand, "price"] } }),
                full_validation,
            );
            // The meta-schema refuses a `$` in a path when registering, but for
            // `$ownerId`, which only a comparison of identifiers may read
            if full_validation && operand.starts_with('$') && operand != "$ownerId" {
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
