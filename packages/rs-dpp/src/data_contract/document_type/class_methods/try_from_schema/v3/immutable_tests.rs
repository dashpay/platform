//! The `immutable` doctype keyword: parser-generation gating and the
//! structural rules.
//!
//! `immutable` joined the grammar at generation 3 (meta-schema v3, protocol
//! version 14) next to `indexOnly`. It is a doctype-level keyword, with the
//! loose-grammar gating that implies on the pre-PV14 side: ignored by earlier
//! generations on the non-validating path (how they treat every doctype
//! keyword they predate) and rejected by their meta-schemas under
//! `full_validation`.
//!
//! The rules themselves (`apply_immutable_fields`) are schema lints, so unlike
//! the indexOnly matrix they only run under `full_validation`; the
//! non-validating path, which stored contracts take, records the list as
//! declared. Only the array shape (an array of strings) is enforced on both
//! paths.

use super::*;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::document_type::accessors::DocumentTypeV2Getters;
use crate::data_contract::errors::DataContractError;
use platform_value::platform_value;
use std::collections::BTreeSet;

/// Parse through this generation with validation mode spelled out.
fn parse_with(
    schema: Value,
    platform_version: &PlatformVersion,
    full_validation: bool,
) -> Result<DocumentTypeV2, ProtocolError> {
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("default config available on this platform version");
    try_from_schema_generation_3(
        Identifier::new([1; 32]),
        1,
        config.version(),
        "post",
        schema,
        None,
        &BTreeMap::new(),
        &config,
        full_validation,
        &mut vec![],
        platform_version,
    )
}

/// Parse through the real dispatcher, which picks the parser generation out
/// of the platform version's `try_from_schema` table value (generation 2 at
/// PV13, generation 3 at PV14).
pub(super) fn parse_dispatched(
    schema: Value,
    platform_version: &PlatformVersion,
    full_validation: bool,
) -> Result<DocumentType, ProtocolError> {
    let config = DataContractConfig::default_for_version(platform_version)
        .expect("default config available on this platform version");
    DocumentType::try_from_schema(
        Identifier::new([1; 32]),
        1,
        config.version(),
        "post",
        schema,
        None,
        &BTreeMap::new(),
        &config,
        full_validation,
        &mut vec![],
        platform_version,
    )
}

/// A mutable `post` type whose `author` is frozen at creation: `body` stays
/// editable, and `meta` is a nested object so the nested-path rule has
/// something to point at.
fn post_schema() -> Value {
    platform_value!({
        "type": "object",
        "documentsMutable": true,
        "properties": {
            "author": { "type": "string", "maxLength": 63, "position": 0 },
            "body": { "type": "string", "maxLength": 500, "position": 1 },
            "meta": {
                "type": "object",
                "position": 2,
                "properties": {
                    "tag": { "type": "string", "maxLength": 30, "position": 0 }
                },
                "additionalProperties": false
            }
        },
        "required": ["author", "body"],
        "immutable": ["author"],
        "additionalProperties": false
    })
}

/// The base schema with one doctype-level key set (added or replaced).
fn post_schema_with(key: &str, value: Value) -> Value {
    let mut schema = post_schema();
    schema.set_value(key, value).expect("doctype key applies");
    schema
}

fn names(entries: &[&str]) -> BTreeSet<String> {
    entries.iter().map(|entry| entry.to_string()).collect()
}

/// The lints surface as `InvalidContractStructure` either directly or, with
/// the `validation` feature on, wrapped as the basic `ContractError`.
pub(super) fn expect_structure_error<T: std::fmt::Debug>(
    result: Result<T, ProtocolError>,
    needle: &str,
) {
    let message = match result {
        Err(ProtocolError::DataContractError(DataContractError::InvalidContractStructure(
            message,
        ))) => message,
        Err(ProtocolError::ConsensusError(boxed)) => match *boxed {
            ConsensusError::BasicError(BasicError::ContractError(
                DataContractError::InvalidContractStructure(message),
            )) => message,
            other => {
                panic!("expected InvalidContractStructure containing {needle:?}, got {other:?}")
            }
        },
        Err(other) => {
            panic!("expected InvalidContractStructure containing {needle:?}, got {other}")
        }
        Ok(parsed) => {
            panic!("expected rejection containing {needle:?}, but the schema parsed: {parsed:?}")
        }
    };
    assert!(
        message.contains(needle),
        "expected structure error containing {needle:?}, got: {message}"
    );
}

// ── the happy path ──────────────────────────────────────────────────────

#[test]
fn immutable_list_parses_on_both_validation_modes() {
    let platform_version = PlatformVersion::latest();

    for full_validation in [false, true] {
        let document_type = parse_with(post_schema(), platform_version, full_validation)
            .unwrap_or_else(|error| {
                panic!("post schema should parse (full_validation: {full_validation}): {error}")
            });

        assert!(document_type.documents_mutable);
        assert_eq!(document_type.immutable_fields(), &names(&["author"]));
    }
}

#[test]
fn omitted_keyword_means_nothing_is_frozen() {
    let mut schema = post_schema();
    let _ = schema.remove_optional_value("immutable");

    let document_type = parse_with(schema, PlatformVersion::latest(), true).expect("schema parses");

    assert!(document_type.immutable_fields().is_empty());
}

#[test]
fn accepts_an_object_property_listed_whole() {
    let schema = post_schema_with("immutable", platform_value!(["author", "meta"]));

    let document_type =
        parse_with(schema, PlatformVersion::latest(), true).expect("object property accepted");

    assert_eq!(
        document_type.immutable_fields(),
        &names(&["author", "meta"])
    );
}

// ── the lints (full validation only) ────────────────────────────────────

#[test]
fn rejects_an_unknown_property() {
    let schema = post_schema_with("immutable", platform_value!(["author", "nope"]));
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), true),
        "\"nope\" as immutable, but it is not a top-level property",
    );
}

#[test]
fn rejects_a_nested_path_with_a_hint() {
    let schema = post_schema_with("immutable", platform_value!(["meta.tag"]));
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), true),
        "nested paths are not accepted",
    );
}

#[test]
fn rejects_a_system_property() {
    let schema = post_schema_with("immutable", platform_value!(["$ownerId"]));
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), true),
        "system property \"$ownerId\"",
    );
}

/// A transient property is never stored, so frozen it could never be
/// written, and required as well it would leave the type permanently
/// uneditable (supplying it fails the immutable check, omitting it fails
/// required-field validation). The grow-only update rule would then keep
/// it that way, so the overlap is refused at registration.
#[test]
fn rejects_a_transient_property() {
    let schema = post_schema_with("transient", platform_value!(["author"]));
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), true),
        "\"author\" as both transient and immutable",
    );
}

#[test]
fn rejects_the_list_on_a_non_mutable_document_type() {
    let schema = post_schema_with("documentsMutable", Value::Bool(false));
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), true),
        "documentsMutable: false",
    );
}

#[test]
fn non_mutable_document_type_without_the_list_still_parses() {
    let mut schema = post_schema_with("documentsMutable", Value::Bool(false));
    let _ = schema.remove_optional_value("immutable");

    let document_type = parse_with(schema, PlatformVersion::latest(), true)
        .expect("an immutable document type without the keyword is the normal case");

    assert!(document_type.immutable_fields().is_empty());
}

#[test]
fn rejects_duplicate_entries_under_full_validation() {
    // `uniqueItems` in meta-schema v3.
    let schema = post_schema_with("immutable", platform_value!(["author", "author"]));
    assert!(
        parse_with(schema, PlatformVersion::latest(), true).is_err(),
        "meta-schema v3 must reject a duplicated immutable entry"
    );
}

// ── the array shape (both validation modes) ─────────────────────────────

#[test]
fn rejects_a_non_array_value_on_both_modes() {
    for full_validation in [false, true] {
        let schema = post_schema_with("immutable", Value::Text("author".to_string()));
        expect_structure_error(
            parse_with(schema, PlatformVersion::latest(), full_validation),
            "must be an array of top-level property names",
        );
    }
}

#[test]
fn rejects_a_non_string_entry_on_both_modes() {
    for full_validation in [false, true] {
        let schema = post_schema_with("immutable", platform_value!(["author", 7]));
        expect_structure_error(
            parse_with(schema, PlatformVersion::latest(), full_validation),
            "every `immutable` entry must be a property name",
        );
    }
}

// ── the stored-contract path ────────────────────────────────────────────

/// Pins that the lints are validation-only: a stored contract is re-parsed
/// without validation and must come back exactly as declared, so a future
/// tightening of the rules can never make a committed contract unreadable.
#[test]
fn non_validating_parse_records_the_list_as_declared() {
    let schema = post_schema_with("immutable", platform_value!(["author", "nope"]));

    let document_type = parse_with(schema, PlatformVersion::latest(), false)
        .expect("the non-validating path records the declaration without judging it");

    assert_eq!(
        document_type.immutable_fields(),
        &names(&["author", "nope"])
    );
}

// ── generation gating ───────────────────────────────────────────────────

#[test]
fn immutable_keyword_is_inert_below_generation_3_without_validation() {
    let platform_version_13 = PlatformVersion::get(13).expect("PV13 exists");

    // Generation 2 ignores unknown doctype keys on the non-validating path,
    // exactly how it treats every keyword it predates, so the parse succeeds
    // and nothing is frozen.
    let document_type = parse_dispatched(post_schema(), platform_version_13, false)
        .expect("generation 2 ignores unknown doctype-level keywords when not validating");
    assert!(
        document_type.immutable_fields().is_empty(),
        "generation 2 must not record immutable properties"
    );

    // Under full validation the v2 meta-schema rejects the unknown key.
    assert!(
        parse_dispatched(post_schema(), platform_version_13, true).is_err(),
        "meta-schema v2 must reject the immutable keyword"
    );
}

#[test]
fn immutable_list_survives_the_dispatcher_at_latest() {
    let document_type = parse_dispatched(post_schema(), PlatformVersion::latest(), true)
        .expect("post schema parses through the dispatcher at PV14");
    assert_eq!(document_type.immutable_fields(), &names(&["author"]));
}

// ── immutableAllowSetting ───────────────────────────────────────────────

/// `post_schema` with `meta` frozen as well and allowed to be set once.
fn post_schema_allowing_meta_once() -> Value {
    let mut schema = post_schema_with("immutable", platform_value!(["author", "meta"]));
    schema
        .set_value("immutableAllowSetting", platform_value!(["meta"]))
        .expect("doctype key applies");
    schema
}

#[test]
fn allow_setting_list_parses_on_both_validation_modes() {
    let platform_version = PlatformVersion::latest();

    for full_validation in [false, true] {
        let document_type = parse_with(
            post_schema_allowing_meta_once(),
            platform_version,
            full_validation,
        )
        .unwrap_or_else(|error| {
            panic!(
                "allow-setting schema should parse (full_validation: {full_validation}): \
                         {error}"
            )
        });

        assert_eq!(
            document_type.immutable_fields(),
            &names(&["author", "meta"])
        );
        assert_eq!(
            document_type.immutable_fields_allow_setting(),
            &names(&["meta"])
        );
    }
}

#[test]
fn omitted_allow_setting_keyword_means_nothing_may_be_set_late() {
    let document_type =
        parse_with(post_schema(), PlatformVersion::latest(), true).expect("schema parses");

    assert!(document_type.immutable_fields_allow_setting().is_empty());
}

#[test]
fn rejects_an_allow_setting_entry_that_is_not_immutable() {
    // `body` is a real property, but not in `immutable`.
    let schema = post_schema_with("immutableAllowSetting", platform_value!(["body"]));
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), true),
        "\"body\" in `immutableAllowSetting`, but it is not in `immutable`",
    );
}

#[test]
fn rejects_an_allow_setting_entry_naming_an_unknown_property() {
    // Unknown to the type, so also absent from `immutable`.
    let schema = post_schema_with("immutableAllowSetting", platform_value!(["nope"]));
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), true),
        "\"nope\" in `immutableAllowSetting`, but it is not in `immutable`",
    );
}

#[test]
fn rejects_a_non_string_allow_setting_entry_on_both_modes() {
    for full_validation in [false, true] {
        let schema = post_schema_with("immutableAllowSetting", platform_value!(["author", 7]));
        expect_structure_error(
            parse_with(schema, PlatformVersion::latest(), full_validation),
            "every `immutableAllowSetting` entry must be a property name",
        );
    }
}

#[test]
fn non_validating_parse_records_the_allow_setting_list_as_declared() {
    // Not in `immutable`; the lint is validation-only.
    let schema = post_schema_with("immutableAllowSetting", platform_value!(["body"]));

    let document_type = parse_with(schema, PlatformVersion::latest(), false)
        .expect("the non-validating path records the declaration without judging it");

    assert_eq!(
        document_type.immutable_fields_allow_setting(),
        &names(&["body"])
    );
}

#[test]
fn allow_setting_keyword_is_inert_below_generation_3_without_validation() {
    let platform_version_13 = PlatformVersion::get(13).expect("PV13 exists");

    let document_type =
        parse_dispatched(post_schema_allowing_meta_once(), platform_version_13, false)
            .expect("generation 2 ignores unknown doctype-level keywords when not validating");
    assert!(document_type.immutable_fields_allow_setting().is_empty());

    assert!(
        parse_dispatched(post_schema_allowing_meta_once(), platform_version_13, true).is_err(),
        "meta-schema v2 must reject the immutableAllowSetting keyword"
    );
}
