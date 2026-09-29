//! `generatedFrom`: the property keyword saying the platform generates a
//! string property with a built-in function of other properties of the same
//! document.
//!
//! The grammar is the v3 document meta-schema's (protocol version 14), the
//! parse is `apply_generated_from` 0 and the checks against the rest of the
//! type are `validate_generated_from_declarations`, both reached from
//! protocol version 14 only. At write time the platform generates a
//! left-out property from its params (`fill_generated_properties`), and
//! `DataContract::validate_document_properties` checks it after the JSON
//! schema (`validate_generated_from_properties`).

use super::typed_array_test_helpers::{
    expect_json_schema_error, expect_structure_error, parse_dispatched,
};
use super::*;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::document_type::methods::{
    DocumentTypeBasicMethods, DocumentTypeV0Methods,
};
use crate::data_contract::document_type::property_constraints::DocumentSystemValues;
use crate::data_contract::document_type::{
    GeneratedFrom, GenerationParam, StringTransformation, SystemFunction,
};
use crate::data_contract::validate_document::DataContractDocumentValidationMethodsV0;
use crate::data_contract::DataContract;
use crate::document::Document;
use crate::validation::SimpleConsensusValidationResult;
use platform_value::platform_value;
use platform_value::string_encoding::Encoding;
use serde_json::json;

const HOMOGRAPH_SAFE_ASCII: &str = "sys.stringTransformations.homographSafeASCII";

fn generated_from(source: &str) -> Value {
    platform_value!({ "function": HOMOGRAPH_SAFE_ASCII, "params": [source] })
}

fn string_property(position: u64) -> Value {
    platform_value!({ "type": "string", "maxLength": 32, "position": position })
}

fn generated_property(position: u64, source: &str) -> Value {
    let mut property = string_property(position);
    property
        .insert("generatedFrom".to_string(), generated_from(source))
        .expect("the property is a map");
    property
}

/// A `handle` type: a top-level `label` and its `normalizedLabel`, an object
/// `profile` holding `display` and its `normalizedDisplay`, and a top-level
/// `slug` generated from the nested `profile.display`.
fn schema() -> Value {
    platform_value!({
        "type": "object",
        "properties": {
            "label": string_property(0),
            "normalizedLabel": generated_property(1, "label"),
            "profile": {
                "type": "object",
                "position": 2,
                "properties": {
                    "display": string_property(0),
                    "normalizedDisplay": generated_property(1, "profile.display")
                },
                "additionalProperties": false
            },
            "slug": generated_property(3, "profile.display")
        },
        "additionalProperties": false
    })
}

/// A document type with a string `label` and the one other property `value`.
fn schema_with(value: Value) -> Value {
    platform_value!({
        "type": "object",
        "properties": { "label": string_property(0), "value": value },
        "additionalProperties": false
    })
}

fn parse(schema: Value) -> DocumentType {
    parse_dispatched(schema, PlatformVersion::latest(), true).expect("the schema parses")
}

fn generated_from_of(document_type: &DocumentType, path: &str) -> Option<GeneratedFrom> {
    document_type
        .as_ref()
        .flattened_properties()
        .get(path)
        .unwrap_or_else(|| panic!("{path} is parsed"))
        .generated_from
        .clone()
}

/// Both parses refuse the declaration: the validating one with the meta-schema
/// (or, for a check the meta-schema cannot state, with the parser's
/// structure error), the one that skips the meta-schema with the parser's.
fn expect_refused(schema: Value, needle: &str) {
    assert!(
        parse_dispatched(schema.clone(), PlatformVersion::latest(), true).is_err(),
        "a validating parse refuses it ({needle})"
    );
    expect_structure_error(
        parse_dispatched(schema, PlatformVersion::latest(), false),
        needle,
    );
}

fn first_basic_error(result: SimpleConsensusValidationResult) -> BasicError {
    match result.errors.into_iter().next() {
        Some(ConsensusError::BasicError(error)) => error,
        other => panic!("expected a basic error, got {other:?}"),
    }
}

// ================================================================
//  Parse
// ================================================================

#[test]
fn should_parse_generated_from_onto_string_properties_at_any_depth() {
    let document_type = parse(schema());
    let expected = |source: &str| {
        Some(GeneratedFrom {
            function: SystemFunction::StringTransformation(
                StringTransformation::HomographSafeAscii,
            ),
            params: vec![GenerationParam::Property(source.to_string())],
        })
    };

    assert_eq!(
        generated_from_of(&document_type, "normalizedLabel"),
        expected("label")
    );
    assert_eq!(
        generated_from_of(&document_type, "profile.normalizedDisplay"),
        expected("profile.display")
    );
    assert_eq!(
        generated_from_of(&document_type, "slug"),
        expected("profile.display")
    );
    assert_eq!(generated_from_of(&document_type, "label"), None);
}

#[test]
fn should_refuse_generated_from_on_a_property_that_is_not_a_string() {
    let schema = schema_with(platform_value!({
        "type": "integer",
        "minimum": 0,
        "maximum": 100,
        "generatedFrom": generated_from("label"),
        "position": 1
    }));
    let error = expect_json_schema_error(parse_dispatched(
        schema.clone(),
        PlatformVersion::latest(),
        true,
    ));
    assert_eq!(error.keyword(), "const", "{error:?}");
    expect_structure_error(
        parse_dispatched(schema, PlatformVersion::latest(), false),
        "generatedFrom is only allowed on string properties",
    );
}

#[test]
fn should_refuse_generated_from_on_a_typed_array_or_its_items() {
    for value in [
        platform_value!({
            "type": "array",
            "maxItems": 4,
            "generatedFrom": generated_from("label"),
            "items": { "type": "string", "maxLength": 16 },
            "position": 1
        }),
        platform_value!({
            "type": "array",
            "maxItems": 4,
            "items": {
                "type": "string",
                "maxLength": 16,
                "generatedFrom": generated_from("label")
            },
            "position": 1
        }),
    ] {
        expect_json_schema_error(parse_dispatched(
            schema_with(value.clone()),
            PlatformVersion::latest(),
            true,
        ));
        expect_structure_error(
            parse_dispatched(schema_with(value), PlatformVersion::latest(), false),
            "not on a typed array or its items",
        );
    }
}

/// A `$ref` replaces every keyword written beside it with its definition, so a
/// declaration there would be dropped: the meta-schema refuses it.
#[test]
fn should_refuse_generated_from_beside_a_ref() {
    let platform_version = PlatformVersion::latest();
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("default config available on this platform version");
    let schema_defs = BTreeMap::from([(
        "name".to_string(),
        platform_value!({ "type": "string", "maxLength": 32 }),
    )]);
    let schema = platform_value!({
        "type": "object",
        "properties": {
            "label": string_property(0),
            "normalizedLabel": {
                "$ref": "#/$defs/name",
                "type": "string",
                "position": 1,
                "generatedFrom": generated_from("label")
            }
        },
        "additionalProperties": false
    });
    let error = expect_json_schema_error(DocumentType::try_from_schema(
        Identifier::new([1; 32]),
        1,
        config.version(),
        "handle",
        schema,
        Some(&schema_defs),
        &BTreeMap::new(),
        &config,
        true,
        &mut vec![],
        platform_version,
    ));
    assert_eq!(error.keyword(), "not", "{error:?}");
}

/// Every system function registers, and generates what it returns for the
/// document's param.
#[test]
fn should_register_and_generate_with_every_string_transformation() {
    for transformation in StringTransformation::ALL {
        let document_type = parse(schema_with(platform_value!({
            "type": "string",
            "maxLength": 64,
            "generatedFrom": { "function": transformation.as_str(), "params": ["label"] },
            "position": 1
        })));
        assert_eq!(
            generated_from_of(&document_type, "value").map(|declaration| declaration.function),
            Some(SystemFunction::StringTransformation(transformation))
        );
        let mut properties = BTreeMap::from([(
            "label".to_string(),
            Value::Text("hello World-again".to_string()),
        )]);
        document_type
            .fill_generated_properties(&mut properties, PlatformVersion::latest())
            .expect("the fill runs");
        assert_eq!(
            properties.get("value"),
            Some(&Value::Text(transformation.apply("hello World-again"))),
            "{}",
            transformation.as_str()
        );
    }
}

#[test]
fn should_refuse_a_malformed_generated_from() {
    let with = |generated_from: Value| {
        schema_with(platform_value!({
            "type": "string",
            "maxLength": 32,
            "generatedFrom": generated_from,
            "position": 1
        }))
    };

    for (declaration, needle) in [
        (
            platform_value!({
                "function": "sys.stringTransformations.homographSafe",
                "params": ["label"]
            }),
            "function \"sys.stringTransformations.homographSafe\" is unknown, expected one of \
             \"sys.stringTransformations.camelCase\", \"sys.stringTransformations.capitalize\", \
             \"sys.stringTransformations.homographSafeASCII\", \"sys.stringTransformations.lowercase\", \
             \"sys.stringTransformations.snakeCase\", \"sys.stringTransformations.uppercase\"",
        ),
        // Built-ins are named under sys.: the bare name is no function
        (
            platform_value!({ "function": "homographSafeASCII", "params": ["label"] }),
            "function \"homographSafeASCII\" is unknown",
        ),
        // A hash is a system function too, but one a refersTo lookup key
        // computes, not one that generates a string
        (
            platform_value!({ "function": "sys.hash.sha256d", "params": ["label"] }),
            "generatedFrom function sys.hash.sha256d does not generate a string",
        ),
        (
            platform_value!({ "params": ["label"] }),
            "generatedFrom must be an object with a function",
        ),
        (
            platform_value!({ "function": HOMOGRAPH_SAFE_ASCII }),
            "generatedFrom must be an object with a function",
        ),
        (
            platform_value!({ "function": HOMOGRAPH_SAFE_ASCII, "params": "label" }),
            "generatedFrom must be an object with a function",
        ),
        (
            platform_value!({ "function": HOMOGRAPH_SAFE_ASCII, "params": ["label"], "extra": 1 }),
            "generatedFrom \"extra\" is unknown, expected function and params",
        ),
        (
            platform_value!({ "function": HOMOGRAPH_SAFE_ASCII, "params": [] }),
            "takes 1 parameter(s), but params lists 0",
        ),
        (
            platform_value!({ "function": HOMOGRAPH_SAFE_ASCII, "params": ["label", "label"] }),
            "takes 1 parameter(s), but params lists 2",
        ),
        (
            platform_value!({ "function": HOMOGRAPH_SAFE_ASCII, "params": [7] }),
            "generatedFrom params must be property paths (strings)",
        ),
        (
            platform_value!({ "function": HOMOGRAPH_SAFE_ASCII, "params": [""] }),
            "generatedFrom params must be between 1 and 256 characters",
        ),
        (
            platform_value!({ "function": HOMOGRAPH_SAFE_ASCII, "params": ["$ownerId"] }),
            "not system property \"$ownerId\"",
        ),
        (
            platform_value!("label"),
            "generatedFrom must be an object with a function",
        ),
    ] {
        expect_refused(with(declaration), needle);
    }
}

#[test]
fn should_refuse_a_param_that_is_not_another_string_property() {
    for (value, needle) in [
        (
            generated_property(1, "missing"),
            "generatedFrom param \"missing\" is not a property of the document type",
        ),
        (
            generated_property(1, "value"),
            "generatedFrom reads the property itself",
        ),
    ] {
        expect_refused(schema_with(value), needle);
    }

    let with_extra = |extra: Value, source: &str| {
        platform_value!({
            "type": "object",
            "properties": {
                "label": string_property(0),
                "extra": extra,
                "value": generated_property(2, source)
            },
            "additionalProperties": false
        })
    };
    expect_refused(
        with_extra(
            platform_value!({ "type": "integer", "minimum": 0, "maximum": 9, "position": 1 }),
            "extra",
        ),
        "generatedFrom param \"extra\" has type",
    );
    expect_refused(
        with_extra(
            platform_value!({
                "type": "object",
                "position": 1,
                "properties": { "inner": string_property(0) },
                "additionalProperties": false
            }),
            "extra",
        ),
        "generatedFrom param \"extra\" is an object, not a string property",
    );
    // A nested member is named by its dotted path
    parse(with_extra(
        platform_value!({
            "type": "object",
            "position": 1,
            "properties": { "inner": string_property(0) },
            "additionalProperties": false
        }),
        "extra.inner",
    ));
}

/// The platform fills every left-out property from a value the client sent,
/// so a param is never itself a generated property.
#[test]
fn should_refuse_a_param_that_is_generated_itself() {
    let schema = platform_value!({
        "type": "object",
        "properties": {
            "label": string_property(0),
            "normalizedLabel": generated_property(1, "label"),
            "twiceNormalized": generated_property(2, "normalizedLabel")
        },
        "additionalProperties": false
    });
    expect_refused(
        schema,
        "generatedFrom param \"normalizedLabel\" is generated itself",
    );
}

#[test]
fn should_refuse_a_transient_property_or_param() {
    for (transient, needle) in [
        (
            "value",
            "is on a property that is transient or inside a transient object",
        ),
        (
            "label",
            "param \"label\" is transient or inside a transient object",
        ),
    ] {
        let mut schema = schema_with(generated_property(1, "label"));
        schema
            .insert("transient".to_string(), platform_value!([transient]))
            .expect("the schema is a map");
        expect_refused(schema, needle);
    }

    // Inside a transient object, on either side
    let with_transient_profile = |slug: Value| {
        platform_value!({
            "type": "object",
            "properties": {
                "profile": {
                    "type": "object",
                    "position": 0,
                    "properties": {
                        "display": string_property(0),
                        "normalizedDisplay": generated_property(1, "profile.display")
                    },
                    "additionalProperties": false
                },
                "plain": string_property(1),
                "slug": slug
            },
            "transient": ["profile"],
            "additionalProperties": false
        })
    };
    let error = parse_dispatched(
        with_transient_profile(generated_property(2, "plain")),
        PlatformVersion::latest(),
        false,
    )
    .expect_err("a generated property inside a transient object is refused");
    assert!(
        error.to_string().contains(
            "\"profile.normalizedDisplay\" generatedFrom is on a property that is transient"
        ),
        "{error}"
    );

    let schema = platform_value!({
        "type": "object",
        "properties": {
            "profile": {
                "type": "object",
                "position": 0,
                "properties": { "display": string_property(0) },
                "additionalProperties": false
            },
            "slug": generated_property(1, "profile.display")
        },
        "transient": ["profile"],
        "additionalProperties": false
    });
    expect_refused(
        schema,
        "generatedFrom param \"profile.display\" is transient or inside a transient object",
    );
}

/// The platform writes a left-out property into the objects around it, which a
/// document supplying the params must hold: every param sits inside every one.
#[test]
fn should_refuse_a_param_outside_the_object_that_holds_the_property() {
    let with_profile = |source: &str| {
        platform_value!({
            "type": "object",
            "properties": {
                "label": string_property(0),
                "profile": {
                    "type": "object",
                    "position": 1,
                    "properties": {
                        "display": string_property(0),
                        "inner": {
                            "type": "object",
                            "position": 1,
                            "properties": { "deep": string_property(0) },
                            "additionalProperties": false
                        },
                        "normalized": generated_property(2, source)
                    },
                    "additionalProperties": false
                }
            },
            "additionalProperties": false
        })
    };

    expect_refused(
        with_profile("label"),
        "generatedFrom param \"label\" is outside \"profile\"",
    );
    // Inside the object, at its level or deeper, is fine
    parse(with_profile("profile.display"));
    parse(with_profile("profile.inner.deep"));
}

#[test]
fn should_ignore_generated_from_before_protocol_version_14() {
    let platform_version = PlatformVersion::get(13).expect("protocol version 13");

    // Protocol version 13's meta-schema refuses the keyword
    expect_json_schema_error(parse_dispatched(schema(), platform_version, true));

    // A parse that skips it (a contract read back from state) ignores it, as it always did
    let document_type = parse_dispatched(schema(), platform_version, false)
        .expect("a parse predating generatedFrom ignores it");
    assert_eq!(generated_from_of(&document_type, "normalizedLabel"), None);
    assert_eq!(generated_from_of(&document_type, "slug"), None);
}

// ================================================================
//  Fill on arrival
// ================================================================

fn data(value: Value) -> BTreeMap<String, Value> {
    value.into_btree_string_map().expect("a map")
}

#[test]
fn should_fill_every_left_out_property_from_its_params() {
    let document_type = parse(schema());
    let mut properties = data(platform_value!({
        "label": "Bob",
        "profile": { "display": "Lil-Olive" }
    }));
    document_type
        .fill_generated_properties(&mut properties, PlatformVersion::latest())
        .expect("the fill runs");

    assert_eq!(
        properties,
        data(platform_value!({
            "label": "Bob",
            "normalizedLabel": "b0b",
            "profile": { "display": "Lil-Olive", "normalizedDisplay": "111-011ve" },
            "slug": "111-011ve"
        }))
    );
}

/// What a client sends is left as it is, right or wrong: the check judges it.
#[test]
fn should_leave_a_supplied_property_and_an_absent_param_alone() {
    let document_type = parse(schema());
    for sent in [
        platform_value!({ "label": "Bob", "normalizedLabel": "wrong" }),
        platform_value!({ "label": "Bob", "normalizedLabel": "b0b" }),
        platform_value!({ "normalizedLabel": "b0b" }),
        platform_value!({ "profile": {} }),
        platform_value!({}),
        // A param that is not a string is the schema's to refuse
        platform_value!({ "label": 7 }),
    ] {
        let mut properties = data(sent.clone());
        document_type
            .fill_generated_properties(&mut properties, PlatformVersion::latest())
            .expect("the fill runs");
        assert_eq!(properties, data(sent.clone()), "{sent:?}");
    }
}

#[test]
fn should_fill_nothing_before_protocol_version_14() {
    // A type parsed with the keyword, filled under protocol version 13's method table
    let document_type = parse(schema());
    let mut properties = data(platform_value!({ "label": "Bob" }));
    document_type
        .fill_generated_properties(
            &mut properties,
            PlatformVersion::get(13).expect("protocol version 13"),
        )
        .expect("the fill runs");
    assert_eq!(properties, data(platform_value!({ "label": "Bob" })));
}

/// The data as the platform stores it: generated properties written into a copy,
/// and the data borrowed as it is on a type that declares none.
#[test]
fn should_read_the_data_as_stored() {
    use std::borrow::Cow;

    let data = data(platform_value!({ "label": "Bob" }));
    let generating = parse(schema());
    let stored = generating
        .data_as_stored(&data, PlatformVersion::latest())
        .expect("the fill runs");
    assert!(matches!(stored, Cow::Owned(_)));
    assert_eq!(
        stored.get("normalizedLabel"),
        Some(&Value::Text("b0b".to_string()))
    );

    let plain = parse(schema_with(string_property(1)));
    assert!(matches!(
        plain
            .data_as_stored(&data, PlatformVersion::latest())
            .expect("nothing to fill"),
        Cow::Borrowed(borrowed) if borrowed == &data
    ));
}

/// The client-side twin sets every property to what its current params generate,
/// replacing a stale value, and removes one whose param is absent.
#[test]
fn should_regenerate_every_property_from_its_current_params() {
    let document_type = parse(schema());
    let mut properties = data(platform_value!({
        "label": "Robin",
        "normalizedLabel": "b0b",
        "profile": { "normalizedDisplay": "stale" },
        "slug": "stale"
    }));
    document_type
        .regenerate_generated_properties(&mut properties, PlatformVersion::latest())
        .expect("the regeneration runs");
    assert_eq!(
        properties,
        data(platform_value!({
            "label": "Robin",
            "normalizedLabel": "r0b1n",
            "profile": {}
        }))
    );
}

// ================================================================
//  Check
// ================================================================

#[test]
fn should_accept_the_generated_value_and_both_absent() {
    let document_type = parse(schema());
    for properties in [
        platform_value!({ "label": "Bob", "normalizedLabel": "b0b" }),
        platform_value!({
            "profile": { "display": "Oil", "normalizedDisplay": "011" },
            "slug": "011"
        }),
        // Characters outside ASCII are kept as they are
        platform_value!({ "label": "Olé", "normalizedLabel": "01é" }),
        platform_value!({ "label": "", "normalizedLabel": "" }),
        platform_value!({}),
        platform_value!({ "profile": {} }),
    ] {
        let result = document_type
            .validate_generated_from_properties(&properties, PlatformVersion::latest())
            .expect("validation executes");
        assert!(result.is_valid(), "{properties:?}: {:?}", result.errors);
    }
}

#[test]
fn should_refuse_a_property_that_is_not_the_generated_value() {
    let document_type = parse(schema());
    for (properties, property, source) in [
        // Not the generated value
        (
            platform_value!({ "label": "Bob", "normalizedLabel": "bob" }),
            "normalizedLabel",
            "label",
        ),
        // Present without its param
        (
            platform_value!({ "normalizedLabel": "b0b" }),
            "normalizedLabel",
            "label",
        ),
        // Absent while its param is present: a document that skipped the fill
        (
            platform_value!({ "label": "Bob" }),
            "normalizedLabel",
            "label",
        ),
        // A nested one is named by its dotted path
        (
            platform_value!({
                "profile": { "display": "Oil", "normalizedDisplay": "oil" },
                "slug": "011"
            }),
            "profile.normalizedDisplay",
            "profile.display",
        ),
    ] {
        let result = document_type
            .validate_generated_from_properties(&properties, PlatformVersion::latest())
            .expect("validation executes");
        match first_basic_error(result) {
            BasicError::DocumentPropertyNotGeneratedError(e) => {
                assert_eq!(e.document_type_name(), "charter", "{properties:?}");
                assert_eq!(e.property(), property, "{properties:?}");
                assert_eq!(e.params(), [source.to_string()], "{properties:?}");
                assert_eq!(e.function(), HOMOGRAPH_SAFE_ASCII, "{properties:?}");
            }
            other => {
                panic!("{properties:?}: expected DocumentPropertyNotGeneratedError, got {other:?}")
            }
        }
    }
}

#[test]
fn should_check_nothing_before_protocol_version_14() {
    // A type parsed with the keyword, judged under protocol version 13's method table
    let document_type = parse(schema());
    assert!(document_type
        .validate_generated_from_properties(
            &platform_value!({ "label": "Bob", "normalizedLabel": "wrong" }),
            PlatformVersion::get(13).expect("protocol version 13"),
        )
        .expect("validation executes")
        .is_valid());
}

/// A repeated key on the way to a param or to the property is refused: the
/// schema validation and the stored document keep the last of repeated keys,
/// where the platform generated from the first.
#[test]
fn should_refuse_a_repeated_key_on_the_way_to_a_param_or_the_property() {
    let document_type = parse(schema());
    let text = |value: &str| Value::Text(value.to_string());
    let map = |entries: Vec<(&str, Value)>| {
        Value::Map(
            entries
                .into_iter()
                .map(|(key, value)| (text(key), value))
                .collect(),
        )
    };

    // A repeated param: the fill reads the first, the stored document would keep the last
    let mut repeated_param = BTreeMap::from([(
        "profile".to_string(),
        map(vec![("display", text("zzz")), ("display", text("B0B"))]),
    )]);
    document_type
        .fill_generated_properties(&mut repeated_param, PlatformVersion::latest())
        .expect("the fill runs");
    assert_eq!(repeated_param.get("slug"), Some(&text("zzz")));

    let repeated_property = BTreeMap::from([
        (
            "profile".to_string(),
            map(vec![
                ("display", text("Bob")),
                ("normalizedDisplay", text("zzz")),
                ("normalizedDisplay", text("b0b")),
            ]),
        ),
        ("slug".to_string(), text("b0b")),
    ]);

    for properties in [repeated_param, repeated_property] {
        let properties = Value::from(properties);
        let result = document_type
            .validate_generated_from_properties(&properties, PlatformVersion::latest())
            .expect("validation executes");
        match first_basic_error(result) {
            BasicError::DocumentPropertyNotGeneratedError(e) => {
                assert_eq!(e.property(), "profile.normalizedDisplay", "{properties:?}")
            }
            other => {
                panic!("{properties:?}: expected DocumentPropertyNotGeneratedError, got {other:?}")
            }
        }
    }
}

// ================================================================
//  Document validation
// ================================================================

fn handle_contract() -> DataContract {
    let schema = serde_json::to_value(schema()).expect("the schema converts to JSON");
    let contract = json!({
        "$formatVersion": "1",
        "id": Identifier::from([7; 32]).to_string(Encoding::Base58),
        "ownerId": Identifier::from([8; 32]).to_string(Encoding::Base58),
        "version": 1,
        "documentSchemas": { "handle": schema }
    });
    DataContract::from_value(
        platform_value::to_value(contract).expect("the contract converts"),
        true,
        PlatformVersion::latest(),
    )
    .expect("the contract parses")
}

/// `validate_document_properties` runs the check after the JSON schema: a
/// schema error keeps precedence, then a property that is not the generated
/// value is refused.
#[test]
fn should_refuse_a_wrong_generated_property_in_document_validation() {
    let platform_version = PlatformVersion::latest();
    let contract = handle_contract();
    let validate = |properties: Value| {
        contract
            .validate_document_properties(
                "handle",
                properties,
                &DocumentSystemValues::default(),
                platform_version,
            )
            .expect("validation returns a consensus result")
    };

    assert!(validate(platform_value!({ "label": "Bob", "normalizedLabel": "b0b" })).is_valid());

    assert!(matches!(
        validate(platform_value!({ "label": "Bob", "normalizedLabel": "bob" })).first_error(),
        Some(ConsensusError::BasicError(BasicError::DocumentPropertyNotGeneratedError(e)))
            if e.property() == "normalizedLabel"
    ));

    // The schema's own refusal comes first
    assert!(matches!(
        validate(platform_value!({ "label": 7, "normalizedLabel": "bob" })).first_error(),
        Some(ConsensusError::BasicError(BasicError::JsonSchemaError(_)))
    ));
}

// ================================================================
//  Client builders and random documents
// ================================================================

/// An immutable `name` type whose unique index over `normalizedLabel` is
/// contested, as DPNS's `domain` is.
fn contested_name_schema() -> Value {
    platform_value!({
        "type": "object",
        "documentsMutable": false,
        "indices": [
            {
                "name": "byNormalizedLabel",
                "properties": [{ "normalizedLabel": "asc" }],
                "unique": true,
                "contested": {
                    "fieldMatches": [
                        { "field": "normalizedLabel", "regexPattern": "^[a-zA-Z01-]{3,19}$" }
                    ],
                    "resolution": 0
                }
            }
        ],
        "properties": {
            "label": string_property(0),
            "normalizedLabel": generated_property(1, "label")
        },
        "required": ["label", "normalizedLabel"],
        "additionalProperties": false
    })
}

/// An indexOnly `entry` type: a `name` and its `normalizedName`, both in the
/// one index, whose terminal is the owner.
fn index_only_entry_schema() -> Value {
    platform_value!({
        "type": "object",
        "indexOnly": true,
        "documentsMutable": false,
        "indices": [
            {
                "name": "byNormalizedName",
                "properties": [{ "normalizedName": "asc" }, { "name": "asc" }],
                "terminal": "$ownerId"
            }
        ],
        "properties": {
            "name": string_property(0),
            "normalizedName": generated_property(1, "name")
        },
        "required": ["name", "normalizedName"],
        "additionalProperties": false
    })
}

fn document_of(document_type: &DocumentType, properties: Value) -> Document {
    document_type
        .as_ref()
        .create_document_from_data(
            properties,
            Identifier::new([3; 32]),
            1,
            1,
            [7; 32],
            PlatformVersion::latest(),
        )
        .expect("the document builds")
}

/// The contest is resolved on the document the platform will store: the create
/// builder generates the property a document leaves out before it resolves the
/// contest, so the transition carries both.
#[test]
fn should_build_a_create_transition_carrying_the_generated_property_and_its_contest() {
    use crate::state_transition::batch_transition::batched_transition::DocumentCreateTransition;
    use crate::state_transition::batch_transition::document_create_transition::v0::v0_methods::DocumentCreateTransitionV0Methods;

    let document_type = parse(contested_name_schema());
    let transition = DocumentCreateTransition::from_document(
        document_of(&document_type, platform_value!({ "label": "Bob" })),
        document_type.as_ref(),
        [7; 32],
        None,
        1,
        PlatformVersion::latest(),
        None,
        None,
    )
    .expect("the create transition builds");

    assert_eq!(
        transition.data().get("normalizedLabel"),
        Some(&Value::Text("b0b".to_string()))
    );
    assert_eq!(
        transition
            .prefunded_voting_balance()
            .as_ref()
            .map(|(index, _)| index.as_str()),
        Some("byNormalizedLabel")
    );
}

#[test]
fn should_build_replace_and_index_only_delete_transitions_carrying_the_generated_property() {
    use crate::state_transition::batch_transition::batched_transition::document_index_only_delete_transition::v0::v0_methods::DocumentIndexOnlyDeleteTransitionV0Methods;
    use crate::state_transition::batch_transition::batched_transition::document_replace_transition::v0::v0_methods::DocumentReplaceTransitionV0Methods;
    use crate::state_transition::batch_transition::batched_transition::{
        DocumentIndexOnlyDeleteTransition, DocumentReplaceTransition,
    };

    let platform_version = PlatformVersion::latest();
    let handle_type = parse(schema());
    let replace = DocumentReplaceTransition::from_document(
        document_of(&handle_type, platform_value!({ "label": "Oil" })),
        handle_type.as_ref(),
        None,
        2,
        platform_version,
        None,
        None,
    )
    .expect("the replace transition builds");
    assert_eq!(
        replace.data().get("normalizedLabel"),
        Some(&Value::Text("011".to_string()))
    );

    // A document fetched and edited still holds the value generated from its old
    // label: the builder replaces it with the one the platform would generate
    let stale = DocumentReplaceTransition::from_document(
        document_of(
            &handle_type,
            platform_value!({ "label": "Robin", "normalizedLabel": "b0b" }),
        ),
        handle_type.as_ref(),
        None,
        2,
        platform_version,
        None,
        None,
    )
    .expect("the replace transition builds");
    assert_eq!(
        stale.data().get("normalizedLabel"),
        Some(&Value::Text("r0b1n".to_string()))
    );

    let entry_type = parse(index_only_entry_schema());
    let delete = DocumentIndexOnlyDeleteTransition::from_document(
        document_of(&entry_type, platform_value!({ "name": "Bob" })),
        entry_type.as_ref(),
        None,
        3,
        platform_version,
        None,
        None,
    )
    .expect("the indexOnly delete transition builds");
    assert_eq!(
        delete.data().get("normalizedName"),
        Some(&Value::Text("b0b".to_string()))
    );
}

/// Random documents hold each generated property as its function returns it for
/// their random params, so fixtures and strategy tests produce documents
/// consensus accepts.
#[cfg(feature = "random-documents")]
#[test]
fn should_generate_random_documents_holding_their_generated_values() {
    use crate::data_contract::document_type::random_document::{
        CreateRandomDocument, DocumentFieldFillSize, DocumentFieldFillType,
    };
    use crate::document::DocumentV0Getters;
    use platform_value::Bytes32;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    let platform_version = PlatformVersion::latest();
    let document_type = parse(schema());
    let mut rng = StdRng::seed_from_u64(99);
    for fill_type in [
        DocumentFieldFillType::FillIfNotRequired,
        DocumentFieldFillType::DoNotFillIfNotRequired,
    ] {
        for _ in 0..20 {
            let entropy = Bytes32::random_with_rng(&mut rng);
            let document = document_type
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    Identifier::new([3; 32]),
                    entropy,
                    fill_type,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("a random document");
            let properties: Value = document.properties().into();
            let result = document_type
                .validate_generated_from_properties(&properties, platform_version)
                .expect("validation executes");
            assert!(result.is_valid(), "{properties:?}: {:?}", result.errors);
        }
    }
}

/// A required generated property whose param is optional, at the top level and
/// inside an object: random documents draw the param too, so the property is
/// never left out.
#[cfg(feature = "random-documents")]
#[test]
fn should_draw_the_optional_params_of_a_required_generated_property() {
    use crate::data_contract::document_type::random_document::{
        CreateRandomDocument, DocumentFieldFillSize, DocumentFieldFillType,
    };
    use crate::document::DocumentV0Getters;
    use platform_value::Bytes32;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    let platform_version = PlatformVersion::latest();
    let document_type = parse(platform_value!({
        "type": "object",
        "properties": {
            "label": string_property(0),
            "normalizedLabel": generated_property(1, "label"),
            "profile": {
                "type": "object",
                "position": 2,
                "properties": {
                    "display": string_property(0),
                    "normalizedDisplay": generated_property(1, "profile.display")
                },
                "required": ["normalizedDisplay"],
                "additionalProperties": false
            }
        },
        "required": ["normalizedLabel", "profile"],
        "additionalProperties": false
    }));
    let mut rng = StdRng::seed_from_u64(7);
    for fill_type in [
        DocumentFieldFillType::FillIfNotRequired,
        DocumentFieldFillType::DoNotFillIfNotRequired,
    ] {
        for _ in 0..20 {
            let entropy = Bytes32::random_with_rng(&mut rng);
            let document = document_type
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    Identifier::new([3; 32]),
                    entropy,
                    fill_type,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("a random document");
            let properties: Value = document.properties().into();
            for path in ["normalizedLabel", "profile.normalizedDisplay"] {
                assert!(
                    matches!(properties.get_optional_value_at_path(path), Ok(Some(_))),
                    "{path} in {properties:?}"
                );
            }
            let result = document_type
                .validate_generated_from_properties(&properties, platform_version)
                .expect("validation executes");
            assert!(result.is_valid(), "{properties:?}: {:?}", result.errors);
        }
    }
}
