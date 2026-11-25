//! Zero-width typed-array element refusal in parser generation 4 (protocol version 15+):
//! a typed array whose element is a fixed-size byte array of zero bytes (`minItems: 0`,
//! `maxItems: 0`) encodes each element in zero bytes, so a two-byte count decodes into
//! `maxItems` separate values. Generation 4 refuses such a property under full validation
//! (registration and update), while keeping contracts registered at protocol version 14
//! loadable when `full_validation` is `false`.
use super::*;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use platform_value::platform_value;

fn parse_at(
    schema: Value,
    platform_version: &PlatformVersion,
    full_validation: bool,
) -> Result<DocumentType, ProtocolError> {
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("default config available");
    DocumentType::try_from_schema(
        Identifier::new([1; 32]),
        1,
        config.version(),
        "test_doc",
        schema,
        None,
        &BTreeMap::new(),
        &config,
        full_validation,
        &mut vec![],
        platform_version,
    )
}

fn schema_with_top_level_typed_array(items: Value) -> Value {
    platform_value!({
        "type": "object",
        "properties": {
            "list": {
                "type": "array",
                "maxItems": 8,
                "items": items,
                "position": 0,
            },
        },
        "additionalProperties": false,
    })
}

fn schema_with_nested_typed_array(items: Value) -> Value {
    platform_value!({
        "type": "object",
        "properties": {
            "meta": {
                "type": "object",
                "position": 0,
                "properties": {
                    "list": {
                        "type": "array",
                        "maxItems": 8,
                        "items": items,
                        "position": 0,
                    },
                },
                "additionalProperties": false,
            },
        },
        "additionalProperties": false,
    })
}

#[test]
fn should_refuse_zero_width_byte_array_elements_at_protocol_version_15_and_accept_at_14() {
    let v14 = PlatformVersion::get(14).expect("protocol version 14 exists");
    let latest = PlatformVersion::latest();
    let zero_width_item = platform_value!({
        "type": "array",
        "byteArray": true,
        "minItems": 0,
        "maxItems": 0,
    });

    for (case, schema, expected_path) in [
        (
            "top-level typed array",
            schema_with_top_level_typed_array(zero_width_item.clone()),
            "list",
        ),
        (
            "nested typed array",
            schema_with_nested_typed_array(zero_width_item),
            "meta.list",
        ),
    ] {
        // Protocol version 14 (generation 3) stays unchanged for replay safety
        parse_at(schema.clone(), v14, true).unwrap_or_else(|e| {
            panic!("{case}: protocol version 14 should keep accepting it: {e}")
        });

        // Protocol version 15+ (generation 4) refuses it under full validation
        let error = parse_at(schema.clone(), latest, true).expect_err(&format!(
            "{case}: protocol version 15 must refuse zero-width elements"
        ));
        assert!(
            matches!(
                &error,
                ProtocolError::ConsensusError(boxed)
                    if matches!(
                        **boxed,
                        ConsensusError::BasicError(BasicError::ContractError(
                            DataContractError::InvalidContractStructure(ref message)
                        )) if message.contains(expected_path)
                            && message.contains("zero-width byte array element")
                    )
            ),
            "{case}: unexpected error: {error:?}"
        );

        // A stored protocol version 14 contract still loads without full validation
        parse_at(schema, latest, false).unwrap_or_else(|e| {
            panic!("{case}: stored contract must still load with full_validation = false: {e}")
        });
    }
}

#[test]
fn should_accept_typed_arrays_whose_elements_encode_to_at_least_one_byte() {
    let latest = PlatformVersion::latest();
    for (case, items) in [
        (
            "variable byte array with minItems 0 and maxItems 1 (length-prefixed)",
            platform_value!({
                "type": "array",
                "byteArray": true,
                "minItems": 0,
                "maxItems": 1,
            }),
        ),
        (
            "variable byte array with only maxItems 0 (length-prefixed)",
            platform_value!({
                "type": "array",
                "byteArray": true,
                "maxItems": 0,
            }),
        ),
        (
            "fixed 1-byte array",
            platform_value!({
                "type": "array",
                "byteArray": true,
                "minItems": 1,
                "maxItems": 1,
            }),
        ),
        (
            "empty-capable string (length-prefixed)",
            platform_value!({
                "type": "string",
                "minLength": 0,
                "maxLength": 0,
            }),
        ),
        ("boolean", platform_value!({ "type": "boolean" })),
    ] {
        parse_at(schema_with_top_level_typed_array(items), latest, true)
            .unwrap_or_else(|e| panic!("{case}: should be accepted: {e}"));
    }
}
