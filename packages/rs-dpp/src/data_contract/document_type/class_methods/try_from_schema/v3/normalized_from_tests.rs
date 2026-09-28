//! `normalizedFrom`: the property keyword saying a string property holds a
//! normalized form of another string property of the same document.
//!
//! The grammar is the v3 document meta-schema's (protocol version 14), the
//! parse is `apply_normalized_from` 0 and the checks against the rest of the
//! type are `validate_normalized_from_declarations`, both reached from
//! protocol version 14 only. At write time the platform fills a left-out
//! property from its source (`fill_normalized_properties`), and
//! `DataContract::validate_document_properties` checks it after the JSON
//! schema (`validate_normalized_from_properties`).

use super::typed_array_test_helpers::{
    expect_json_schema_error, expect_structure_error, parse_dispatched,
};
use super::*;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use crate::data_contract::document_type::methods::DocumentTypeBasicMethods;
use crate::data_contract::document_type::property_constraints::DocumentSystemValues;
use crate::data_contract::document_type::{NormalizationTransform, NormalizedFrom};
use crate::data_contract::validate_document::DataContractDocumentValidationMethodsV0;
use crate::data_contract::DataContract;
use crate::validation::SimpleConsensusValidationResult;
use platform_value::platform_value;
use platform_value::string_encoding::Encoding;
use serde_json::json;

const HOMOGRAPH_SAFE_ASCII: &str = "homographSafeASCII";

fn normalized_from(source: &str) -> Value {
    platform_value!({ "property": source, "transform": HOMOGRAPH_SAFE_ASCII })
}

fn string_property(position: u64) -> Value {
    platform_value!({ "type": "string", "maxLength": 32, "position": position })
}

fn normalized_property(position: u64, source: &str) -> Value {
    let mut property = string_property(position);
    property
        .insert("normalizedFrom".to_string(), normalized_from(source))
        .expect("the property is a map");
    property
}

/// A `handle` type: a top-level `label` and its `normalizedLabel`, an object
/// `profile` holding `display` and its `normalizedDisplay`, and a top-level
/// `slug` normalized from the nested `profile.display`.
fn schema() -> Value {
    platform_value!({
        "type": "object",
        "properties": {
            "label": string_property(0),
            "normalizedLabel": normalized_property(1, "label"),
            "profile": {
                "type": "object",
                "position": 2,
                "properties": {
                    "display": string_property(0),
                    "normalizedDisplay": normalized_property(1, "profile.display")
                },
                "additionalProperties": false
            },
            "slug": normalized_property(3, "profile.display")
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

fn normalized_from_of(document_type: &DocumentType, path: &str) -> Option<NormalizedFrom> {
    document_type
        .as_ref()
        .flattened_properties()
        .get(path)
        .unwrap_or_else(|| panic!("{path} is parsed"))
        .normalized_from
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
fn should_parse_normalized_from_onto_string_properties_at_any_depth() {
    let document_type = parse(schema());
    let expected = |source: &str| {
        Some(NormalizedFrom {
            property: source.to_string(),
            transform: NormalizationTransform::HomographSafeAscii,
        })
    };

    assert_eq!(
        normalized_from_of(&document_type, "normalizedLabel"),
        expected("label")
    );
    assert_eq!(
        normalized_from_of(&document_type, "profile.normalizedDisplay"),
        expected("profile.display")
    );
    assert_eq!(
        normalized_from_of(&document_type, "slug"),
        expected("profile.display")
    );
    assert_eq!(normalized_from_of(&document_type, "label"), None);
}

#[test]
fn should_refuse_normalized_from_on_a_property_that_is_not_a_string() {
    let schema = schema_with(platform_value!({
        "type": "integer",
        "minimum": 0,
        "maximum": 100,
        "normalizedFrom": normalized_from("label"),
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
        "normalizedFrom is only allowed on string properties",
    );
}

#[test]
fn should_refuse_normalized_from_on_a_typed_array_or_its_items() {
    for value in [
        platform_value!({
            "type": "array",
            "maxItems": 4,
            "normalizedFrom": normalized_from("label"),
            "items": { "type": "string", "maxLength": 16 },
            "position": 1
        }),
        platform_value!({
            "type": "array",
            "maxItems": 4,
            "items": {
                "type": "string",
                "maxLength": 16,
                "normalizedFrom": normalized_from("label")
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

#[test]
fn should_refuse_a_malformed_normalized_from() {
    let with = |normalized_from: Value| {
        schema_with(platform_value!({
            "type": "string",
            "maxLength": 32,
            "normalizedFrom": normalized_from,
            "position": 1
        }))
    };

    for (declaration, needle) in [
        (
            platform_value!({ "property": "label", "transform": "homographSafe" }),
            "transform \"homographSafe\" is unknown, expected one of \"homographSafeASCII\"",
        ),
        (
            platform_value!({ "transform": HOMOGRAPH_SAFE_ASCII }),
            "normalizedFrom must be an object with a property",
        ),
        (
            platform_value!({ "property": "label" }),
            "normalizedFrom must be an object with a property",
        ),
        (
            platform_value!({ "property": "label", "transform": HOMOGRAPH_SAFE_ASCII, "extra": 1 }),
            "normalizedFrom \"extra\" is unknown, expected property and transform",
        ),
        (
            platform_value!({ "property": "", "transform": HOMOGRAPH_SAFE_ASCII }),
            "normalizedFrom property must be between 1 and 256 characters",
        ),
        (
            platform_value!({ "property": "$ownerId", "transform": HOMOGRAPH_SAFE_ASCII }),
            "not system property \"$ownerId\"",
        ),
        (
            platform_value!("label"),
            "normalizedFrom must be an object with a property",
        ),
    ] {
        expect_refused(with(declaration), needle);
    }
}

#[test]
fn should_refuse_a_source_that_is_not_another_string_property() {
    for (value, needle) in [
        (
            normalized_property(1, "missing"),
            "normalizedFrom property \"missing\" is not a property of the document type",
        ),
        (
            normalized_property(1, "value"),
            "normalizedFrom names the property itself",
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
                "value": normalized_property(2, source)
            },
            "additionalProperties": false
        })
    };
    expect_refused(
        with_extra(
            platform_value!({ "type": "integer", "minimum": 0, "maximum": 9, "position": 1 }),
            "extra",
        ),
        "normalizedFrom property \"extra\" has type",
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
        "normalizedFrom property \"extra\" is an object, not a string property",
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
/// so a source is never itself a normalized property.
#[test]
fn should_refuse_a_source_that_is_normalized_itself() {
    let schema = platform_value!({
        "type": "object",
        "properties": {
            "label": string_property(0),
            "normalizedLabel": normalized_property(1, "label"),
            "twiceNormalized": normalized_property(2, "normalizedLabel")
        },
        "additionalProperties": false
    });
    expect_refused(
        schema,
        "normalizedFrom property \"normalizedLabel\" declares normalizedFrom itself",
    );
}

#[test]
fn should_refuse_a_transient_property_or_source() {
    for (transient, needle) in [
        (
            "value",
            "is on a property that is transient or inside a transient object",
        ),
        (
            "label",
            "property \"label\" is transient or inside a transient object",
        ),
    ] {
        let mut schema = schema_with(normalized_property(1, "label"));
        schema
            .insert("transient".to_string(), platform_value!([transient]))
            .expect("the schema is a map");
        expect_refused(schema, needle);
    }

    // Inside a transient object, on either side
    let with_transient_profile = |normalized: Value| {
        platform_value!({
            "type": "object",
            "properties": {
                "profile": {
                    "type": "object",
                    "position": 0,
                    "properties": {
                        "display": string_property(0),
                        "normalizedDisplay": normalized_property(1, "profile.display")
                    },
                    "additionalProperties": false
                },
                "plain": string_property(1),
                "slug": normalized
            },
            "transient": ["profile"],
            "additionalProperties": false
        })
    };
    let error = parse_dispatched(
        with_transient_profile(normalized_property(2, "plain")),
        PlatformVersion::latest(),
        false,
    )
    .expect_err("a normalized property inside a transient object is refused");
    assert!(
        error.to_string().contains(
            "\"profile.normalizedDisplay\" normalizedFrom is on a property that is transient"
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
            "slug": normalized_property(1, "profile.display")
        },
        "transient": ["profile"],
        "additionalProperties": false
    });
    expect_refused(
        schema,
        "normalizedFrom property \"profile.display\" is transient or inside a transient object",
    );
}

/// The platform writes a left-out property into the objects around it, which a
/// document supplying the source must hold: the source sits inside every one.
#[test]
fn should_refuse_a_source_outside_the_object_that_holds_the_property() {
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
                        "normalized": normalized_property(2, source)
                    },
                    "additionalProperties": false
                }
            },
            "additionalProperties": false
        })
    };

    expect_refused(
        with_profile("label"),
        "normalizedFrom property \"label\" is outside \"profile\"",
    );
    // Inside the object, at its level or deeper, is fine
    parse(with_profile("profile.display"));
    parse(with_profile("profile.inner.deep"));
}

#[test]
fn should_ignore_normalized_from_before_protocol_version_14() {
    let platform_version = PlatformVersion::get(13).expect("protocol version 13");

    // Protocol version 13's meta-schema refuses the keyword
    expect_json_schema_error(parse_dispatched(schema(), platform_version, true));

    // A parse that skips it (a contract read back from state) ignores it, as it always did
    let document_type = parse_dispatched(schema(), platform_version, false)
        .expect("a parse predating normalizedFrom ignores it");
    assert_eq!(normalized_from_of(&document_type, "normalizedLabel"), None);
    assert_eq!(normalized_from_of(&document_type, "slug"), None);
}

// ================================================================
//  Fill on arrival
// ================================================================

fn data(value: Value) -> BTreeMap<String, Value> {
    value.into_btree_string_map().expect("a map")
}

#[test]
fn should_fill_every_left_out_property_from_its_source() {
    let document_type = parse(schema());
    let mut properties = data(platform_value!({
        "label": "Bob",
        "profile": { "display": "Lil-Olive" }
    }));
    document_type
        .fill_normalized_properties(&mut properties, PlatformVersion::latest())
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
fn should_leave_a_supplied_property_and_an_absent_source_alone() {
    let document_type = parse(schema());
    for sent in [
        platform_value!({ "label": "Bob", "normalizedLabel": "wrong" }),
        platform_value!({ "label": "Bob", "normalizedLabel": "b0b" }),
        platform_value!({ "normalizedLabel": "b0b" }),
        platform_value!({ "profile": {} }),
        platform_value!({}),
        // A source that is not a string is the schema's to refuse
        platform_value!({ "label": 7 }),
    ] {
        let mut properties = data(sent.clone());
        document_type
            .fill_normalized_properties(&mut properties, PlatformVersion::latest())
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
        .fill_normalized_properties(
            &mut properties,
            PlatformVersion::get(13).expect("protocol version 13"),
        )
        .expect("the fill runs");
    assert_eq!(properties, data(platform_value!({ "label": "Bob" })));
}

// ================================================================
//  Check
// ================================================================

#[test]
fn should_accept_the_normalized_form_of_the_source_and_both_absent() {
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
            .validate_normalized_from_properties(&properties, PlatformVersion::latest())
            .expect("validation executes");
        assert!(result.is_valid(), "{properties:?}: {:?}", result.errors);
    }
}

#[test]
fn should_refuse_a_property_that_is_not_its_source_normalized_form() {
    let document_type = parse(schema());
    for (properties, property, source) in [
        // Not the normalized form
        (
            platform_value!({ "label": "Bob", "normalizedLabel": "bob" }),
            "normalizedLabel",
            "label",
        ),
        // Present without its source
        (
            platform_value!({ "normalizedLabel": "b0b" }),
            "normalizedLabel",
            "label",
        ),
        // Absent while its source is present: a document that skipped the fill
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
            .validate_normalized_from_properties(&properties, PlatformVersion::latest())
            .expect("validation executes");
        match first_basic_error(result) {
            BasicError::DocumentPropertyNotNormalizedError(e) => {
                assert_eq!(e.document_type_name(), "charter", "{properties:?}");
                assert_eq!(e.property(), property, "{properties:?}");
                assert_eq!(e.source_property(), source, "{properties:?}");
                assert_eq!(e.transform(), HOMOGRAPH_SAFE_ASCII, "{properties:?}");
            }
            other => {
                panic!("{properties:?}: expected DocumentPropertyNotNormalizedError, got {other:?}")
            }
        }
    }
}

#[test]
fn should_check_nothing_before_protocol_version_14() {
    // A type parsed with the keyword, judged under protocol version 13's method table
    let document_type = parse(schema());
    assert!(document_type
        .validate_normalized_from_properties(
            &platform_value!({ "label": "Bob", "normalizedLabel": "wrong" }),
            PlatformVersion::get(13).expect("protocol version 13"),
        )
        .expect("validation executes")
        .is_valid());
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
/// schema error keeps precedence, then a property that is not its source's
/// normalized form is refused.
#[test]
fn should_refuse_a_wrong_normalized_property_in_document_validation() {
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
        Some(ConsensusError::BasicError(BasicError::DocumentPropertyNotNormalizedError(e)))
            if e.property() == "normalizedLabel"
    ));

    // The schema's own refusal comes first
    assert!(matches!(
        validate(platform_value!({ "label": 7, "normalizedLabel": "bob" })).first_error(),
        Some(ConsensusError::BasicError(BasicError::JsonSchemaError(_)))
    ));
}
