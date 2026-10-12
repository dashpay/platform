//! Rules reading byte arrays under generation 3 (protocol version 14): a
//! `byteAt` operand reads one byte of a byte array property, and `startsWith`
//! and `endsWith` test a byte array property against a hex constant or another
//! byte array property. Every read names a stored byte array property, within
//! its `maxItems` when it declares one, on both paths; the meta-schema checks
//! the shape when a contract registers; the condition of an `immutable` entry,
//! `retractedWhen` and `deleteConstraints` read bytes too; and protocol version
//! 13 refuses the rules when registering.

use super::immutable_tests::{expect_structure_error, parse_dispatched};
use super::*;
use crate::consensus::basic::basic_error::BasicError;
use crate::consensus::basic::document::PropertyConstraintViolation;
use crate::data_contract::config::moderation::{ContractModerationConfig, ContractModerators};
use crate::data_contract::document_type::accessors::DocumentTypeV2Getters;
use crate::data_contract::document_type::property_constraints::{
    DocumentSystemValues, PropertyRead, STORED_DOCUMENT_KEY,
};
use platform_value::platform_value;
use serde_json::json;

/// A `payment` type: a required `price`, an optional 21-byte
/// `corePaymentAddress`, a `payload` of at most 8 bytes, a `blob` with no
/// `maxItems`, a string `note`, an identifier `ownerRef`, and a `meta` object
/// holding a `key` of at most 4 bytes. `extra` is merged in at the top level.
fn payment_schema(extra: serde_json::Value) -> Value {
    let mut schema = json!({
        "type": "object",
        "documentsMutable": true,
        "properties": {
            "price": { "type": "integer", "minimum": 0, "position": 0 },
            "corePaymentAddress": {
                "type": "array",
                "byteArray": true,
                "minItems": 21,
                "maxItems": 21,
                "position": 1
            },
            "payload": { "type": "array", "byteArray": true, "maxItems": 8, "position": 2 },
            "blob": { "type": "array", "byteArray": true, "position": 3 },
            "note": { "type": "string", "maxLength": 63, "position": 4 },
            "ownerRef": {
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "maxItems": 32,
                "contentMediaType": "application/x.dash.dpp.identifier",
                "position": 5
            },
            "meta": {
                "type": "object",
                "position": 6,
                "properties": {
                    "key": { "type": "array", "byteArray": true, "maxItems": 4, "position": 0 }
                },
                "additionalProperties": false
            }
        },
        "required": ["price"],
        "additionalProperties": false
    });
    if let serde_json::Value::Object(entries) = extra {
        for (key, value) in entries {
            schema[key] = value;
        }
    }
    platform_value::to_value(schema).expect("the schema converts")
}

fn parse_payment(
    extra: serde_json::Value,
    full_validation: bool,
) -> Result<DocumentType, ProtocolError> {
    parse_dispatched(
        payment_schema(extra),
        PlatformVersion::latest(),
        full_validation,
    )
}

fn parse_rule(
    rule: serde_json::Value,
    full_validation: bool,
) -> Result<DocumentType, ProtocolError> {
    parse_payment(
        json!({ "propertyConstraints": { "rule": rule } }),
        full_validation,
    )
}

fn is_json_schema_error(error: &ProtocolError) -> bool {
    matches!(
        error,
        ProtocolError::ConsensusError(boxed)
            if matches!(**boxed, ConsensusError::BasicError(BasicError::JsonSchemaError(_)))
    )
}

/// The rules a byte array's bytes allow parse onto the type on both paths,
/// each read reported with the length it needs. A byte array without
/// `maxItems` may be read at any index.
#[test]
fn should_read_the_bytes_of_byte_array_properties_on_both_paths() {
    let rules = json!({
        "addressType": { "in": [{ "byteAt": ["corePaymentAddress", 0] }, [0, 1]] },
        "lastAddressByte": { "lessThan": [{ "byteAt": ["corePaymentAddress", 20] }, 128] },
        "magic": { "startsWith": ["payload", { "const": "CAFE" }] },
        "keyed": { "endsWith": ["payload", "meta.key"] },
        "farIntoBlob": { "notEqual": [{ "byteAt": ["blob", 65535] }, "price"] },
        "fullPayload": { "startsWith": [{ "const": "0102030405060708" }, "payload"] }
    });
    for full_validation in [true, false] {
        let document_type = parse_payment(
            json!({ "propertyConstraints": rules.clone() }),
            full_validation,
        )
        .unwrap_or_else(|e| panic!("full_validation {full_validation}: should parse: {e}"));
        let constraints = document_type.property_constraints();
        assert_eq!(
            constraints["addressType"].property_reads(),
            [("corePaymentAddress", PropertyRead::Bytes { extent: 1 })]
        );
        assert_eq!(
            constraints["lastAddressByte"].property_reads(),
            [("corePaymentAddress", PropertyRead::Bytes { extent: 21 })]
        );
        assert_eq!(
            constraints["magic"].property_reads(),
            [("payload", PropertyRead::Bytes { extent: 2 })]
        );
        assert_eq!(
            constraints["keyed"].property_reads(),
            [
                ("payload", PropertyRead::Bytes { extent: 0 }),
                ("meta.key", PropertyRead::Bytes { extent: 0 })
            ]
        );
        assert_eq!(
            constraints["farIntoBlob"].property_reads(),
            [
                ("blob", PropertyRead::Bytes { extent: 65536 }),
                ("price", PropertyRead::Value)
            ]
        );
        assert_eq!(
            constraints["fullPayload"].property_reads(),
            [("payload", PropertyRead::Bytes { extent: 0 })]
        );
    }
}

/// A read names a stored byte array property: not a string, an identifier, an
/// object or a property the type does not declare; not one inside its
/// `maxItems`'s reach; not a transient one. An integer read of a byte array
/// points at `byteAt`.
#[test]
fn should_hold_a_byte_read_to_a_stored_byte_array_within_its_max_items() {
    for (rule, needle) in [
        (
            json!({ "equal": [{ "byteAt": ["corePaymentAddress", 21] }, 0] }),
            "rule \"rule\" reads \"corePaymentAddress\" as at least 22 bytes long, but its \
             maxItems is 21: the byte a byteAt reads, or the constant it must start or end \
             with, is never there",
        ),
        (
            json!({ "startsWith": ["payload", { "const": "010203040506070809" }] }),
            "rule \"rule\" reads \"payload\" as at least 9 bytes long, but its maxItems is 8",
        ),
        (
            json!({ "endsWith": ["meta.key", { "const": "0001020304" }] }),
            "rule \"rule\" reads \"meta.key\" as at least 5 bytes long, but its maxItems is 4",
        ),
        (
            json!({ "equal": [{ "byteAt": ["note", 0] }, 0] }),
            "rule \"rule\" reads the bytes of \"note\", which has type string, not byteArray",
        ),
        (
            json!({ "equal": [{ "byteAt": ["ownerRef", 0] }, 0] }),
            "rule \"rule\" reads the bytes of \"ownerRef\", which has type identifier, not \
             byteArray: an identifier property is compared whole, by equal, notEqual or in",
        ),
        (
            json!({ "equal": [{ "byteAt": ["meta", 0] }, 0] }),
            "rule \"rule\" reads the bytes of \"meta\", which is not a byte array property of \
             the document type",
        ),
        (
            json!({ "equal": [{ "byteAt": ["missing", 0] }, 0] }),
            "rule \"rule\" reads the bytes of \"missing\", which is not a byte array property",
        ),
        (
            json!({ "lessThan": ["payload", 5] }),
            "rule \"rule\" reads \"payload\", which has type byteArray, not integer or boolean: \
             byteAt reads one of its bytes, and count how many it holds",
        ),
        (
            json!({ "in": ["payload", ["a", "b"]] }),
            "rule \"rule\" compares \"payload\" with a string, but it has type byteArray, not \
             string",
        ),
        (
            json!({ "contains": ["payload", 1] }),
            "rule \"rule\" looks in \"payload\", which has type byteArray, not an array with \
             items",
        ),
    ] {
        for full_validation in [true, false] {
            expect_structure_error(parse_rule(rule.clone(), full_validation), needle);
        }
    }

    // Only a condition judging a replace reads the stored document; the
    // meta-schema refuses the path when registering, so the parser alone is
    // left to refuse it in a stored contract
    expect_structure_error(
        parse_rule(
            json!({ "equal": [{ "byteAt": ["$old.payload", 0] }, 0] }),
            false,
        ),
        "rule \"rule\" reads \"$old.payload\", but only a condition judging a replace",
    );
    expect_structure_error(
        parse_payment(
            json!({
                "deleteConstraints": {
                    "rule": { "startsWith": ["$old.payload", { "const": "ff" }] }
                }
            }),
            false,
        ),
        "rule \"rule\" reads \"$old.payload\", but only a condition judging a replace",
    );

    // A transient byte array is never stored, so no rule may read one
    for rule in [
        json!({ "equal": [{ "byteAt": ["payload", 0] }, 1] }),
        json!({ "startsWith": ["payload", { "const": "00" }] }),
        json!({ "endsWith": ["corePaymentAddress", "payload"] }),
    ] {
        for full_validation in [true, false] {
            expect_structure_error(
                parse_payment(
                    json!({
                        "propertyConstraints": { "rule": rule.clone() },
                        "transient": ["payload"]
                    }),
                    full_validation,
                ),
                "rule \"rule\" reads the bytes of \"payload\", which is transient or inside a \
                 transient object",
            );
        }
    }
    for full_validation in [true, false] {
        expect_structure_error(
            parse_payment(
                json!({
                    "propertyConstraints": {
                        "rule": { "equal": [{ "byteAt": ["meta.key", 0] }, 1] }
                    },
                    "transient": ["meta"]
                }),
                full_validation,
            ),
            "rule \"rule\" reads the bytes of \"meta.key\", which is transient or inside a \
             transient object",
        );
    }
}

/// The meta-schema checks the shape of a `byteAt` when a contract registers;
/// a stored contract is read by the parser alone, which refuses an index out
/// of range too.
#[test]
fn should_check_the_shape_of_byte_reads_with_the_meta_schema_and_the_parser() {
    for rule in [
        json!({ "equal": [{ "byteAt": ["payload"] }, 0] }),
        json!({ "equal": [{ "byteAt": ["payload", 0, 1] }, 0] }),
        json!({ "equal": [{ "byteAt": ["payload", -1] }, 0] }),
        json!({ "equal": [{ "byteAt": ["payload", 65536] }, 0] }),
        json!({ "equal": [{ "byteAt": ["payload", "0"] }, 0] }),
        json!({ "equal": [{ "byteAt": ["$id", 0] }, 0] }),
        json!({ "startsWith": ["payload", { "const": 7 }] }),
    ] {
        let registered = parse_rule(rule.clone(), true);
        assert!(
            registered.as_ref().is_err_and(is_json_schema_error),
            "{rule}: the meta-schema should refuse it, got {registered:?}"
        );
    }

    // A stored contract is read without the meta-schema: the parser refuses
    // the index itself
    expect_structure_error(
        parse_rule(
            json!({ "equal": [{ "byteAt": ["payload", -1] }, 0] }),
            false,
        ),
        "at equal[0].byteAt[1] holds -1, but the index of a byte is an integer from 0 to 65535",
    );

    // What the meta-schema cannot tell, the parser refuses on both paths
    for (rule, needle) in [
        (
            json!({ "startsWith": ["payload", { "const": "abc" }] }),
            "at startsWith[1].const holds \"abc\", which is not an even number of hex digits",
        ),
        (
            json!({ "equal": ["payload", { "const": "00" }] }),
            "at equal compares a byte array property, which only startsWith and endsWith test",
        ),
    ] {
        for full_validation in [true, false] {
            expect_structure_error(parse_rule(rule.clone(), full_validation), needle);
        }
    }
}

/// The rules arrive with protocol version 14: meta-schema v2 closes the
/// document type, so version 13 refuses a contract declaring them when it
/// registers, and its parser ignores them when reading.
#[test]
fn should_refuse_byte_reads_before_protocol_version_14() {
    let schema = payment_schema(json!({
        "propertyConstraints": {
            "addressType": { "in": [{ "byteAt": ["corePaymentAddress", 0] }, [0, 1]] },
            "magic": { "startsWith": ["payload", { "const": "cafe" }] }
        }
    }));
    let platform_version_13 = PlatformVersion::get(13).expect("protocol version 13");

    let registered = parse_dispatched(schema.clone(), platform_version_13, true);
    assert!(
        registered.as_ref().is_err_and(is_json_schema_error),
        "{registered:?}"
    );
    let read = parse_dispatched(schema.clone(), platform_version_13, false)
        .expect("protocol version 13 reads the rest of the type");
    assert!(read.property_constraints().is_empty());

    let parsed = parse_dispatched(schema, PlatformVersion::latest(), true).expect("parses");
    assert_eq!(parsed.property_constraints().len(), 2);
}

/// The condition of an `immutable` entry reads the bytes of the stored
/// document through `$old.`, and `deleteConstraints` the stored document's:
/// both under the same checks.
#[test]
fn should_read_bytes_in_immutable_conditions_and_delete_constraints() {
    let extra = json!({
        "immutable": [{
            "property": "corePaymentAddress",
            "when": { "equal": [{ "byteAt": ["$old.corePaymentAddress", 0] }, 1] }
        }],
        "deleteConstraints": {
            "notLocked": { "not": { "startsWith": ["payload", { "const": "ff" }] } }
        }
    });
    for full_validation in [true, false] {
        let document_type = parse_payment(extra.clone(), full_validation)
            .unwrap_or_else(|e| panic!("full_validation {full_validation}: should parse: {e}"));
        let condition = &document_type.immutable_field_conditions()["corePaymentAddress"];
        assert_eq!(
            condition.property_reads(),
            [("$old.corePaymentAddress", PropertyRead::Bytes { extent: 1 })]
        );
        // Frozen while the stored address is of type 1
        let written = |stored_type: u8| {
            let mut stored = vec![stored_type];
            stored.extend([0; 20]);
            Value::Map(vec![
                (Value::Text("price".to_string()), Value::U64(1)),
                (
                    Value::Text(STORED_DOCUMENT_KEY.to_string()),
                    Value::Map(vec![(
                        Value::Text("corePaymentAddress".to_string()),
                        Value::Bytes(stored),
                    )]),
                ),
            ])
        };
        let system = DocumentSystemValues::default();
        assert_eq!(condition.holds(&written(1), &system), Ok(true));
        assert_eq!(condition.holds(&written(0), &system), Ok(false));

        let delete_rule = &document_type.delete_constraints()["notLocked"];
        assert_eq!(
            delete_rule.violation(
                &platform_value!({ "payload": Value::Bytes(vec![0xff, 1]) }),
                &system
            ),
            Some(PropertyConstraintViolation::NotMet)
        );
        assert_eq!(
            delete_rule.violation(
                &platform_value!({ "payload": Value::Bytes(vec![1]) }),
                &system
            ),
            None
        );
    }

    // Held to the same checks
    for (extra, needle) in [
        (
            json!({
                "immutable": [{
                    "property": "corePaymentAddress",
                    "when": { "equal": [{ "byteAt": ["$old.corePaymentAddress", 21] }, 1] }
                }]
            }),
            "reads \"$old.corePaymentAddress\" as at least 22 bytes long, but its maxItems is 21",
        ),
        (
            json!({
                "deleteConstraints": {
                    "rule": { "startsWith": ["payload", { "const": "000102030405060708" }] }
                }
            }),
            "rule \"rule\" reads \"payload\" as at least 9 bytes long, but its maxItems is 8",
        ),
    ] {
        for full_validation in [true, false] {
            expect_structure_error(parse_payment(extra.clone(), full_validation), needle);
        }
    }
}

/// `retractedWhen` reads bytes like any condition judging a replace: here a
/// post is retracted by writing a payload starting with 0x00.
#[test]
fn should_read_bytes_in_retracted_when() {
    let config = DataContractConfig::default_for_version(PlatformVersion::latest())
        .expect("default config available")
        .with_moderation(Some(ContractModerationConfig {
            banlist: true,
            suspensions: false,
            moderators: ContractModerators::ContractOwner,
            warnings: false,
        }));
    let mut schema = payment_schema(json!({ "canBeDeleted": false }));
    schema
        .set_value(
            "retractedWhen",
            platform_value!({ "startsWith": ["payload", { "const": "00" }] }),
        )
        .expect("doctype key applies");
    for full_validation in [true, false] {
        let document_type = DocumentType::try_from_schema(
            Identifier::new([1; 32]),
            1,
            config.version(),
            "post",
            schema.clone(),
            None,
            &BTreeMap::new(),
            &config,
            full_validation,
            &mut vec![],
            PlatformVersion::latest(),
        )
        .unwrap_or_else(|e| panic!("full_validation {full_validation}: should parse: {e}"));
        let condition = document_type
            .retracted_when()
            .expect("expected a retractedWhen condition");
        let system = DocumentSystemValues::default();
        assert_eq!(
            condition.holds(
                &platform_value!({ "payload": Value::Bytes(vec![0, 9]) }),
                &system
            ),
            Ok(true)
        );
        assert_eq!(
            condition.holds(&platform_value!({ "price": 1u64 }), &system),
            Ok(false)
        );
    }
}
