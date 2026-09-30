//! The `immutableAfter` doctype keyword: its shape, the registration rules
//! and parser-generation gating.
//!
//! `immutableAfter` maps top-level properties of a mutable document type to a
//! window in seconds after the document's `$createdAt`, past which a replace
//! may no longer change them. Like `immutable` beside it, it joined the grammar
//! at generation 3 (meta-schema v3, protocol version 14). Its shape (an object
//! of whole seconds, 1 to `u32::MAX`) is enforced on both validation paths;
//! the rules relating it to the rest of the type (`apply_immutable_after`) are
//! schema lints, run under `full_validation` only.

use super::immutable_tests::{expect_structure_error, parse_dispatched};
use super::*;
use crate::data_contract::document_type::accessors::DocumentTypeV2Getters;
use platform_value::platform_value;

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

/// A mutable `post` whose `text` may be edited for five minutes after it is
/// created, `author` frozen at creation, and `meta` a nested object so the
/// nested-path rule has something to point at.
fn post_schema() -> Value {
    platform_value!({
        "type": "object",
        "documentsMutable": true,
        "properties": {
            "author": { "type": "string", "maxLength": 63, "position": 0 },
            "text": { "type": "string", "maxLength": 500, "position": 1 },
            "meta": {
                "type": "object",
                "position": 2,
                "properties": {
                    "tag": { "type": "string", "maxLength": 30, "position": 0 }
                },
                "additionalProperties": false
            }
        },
        "required": ["author", "text", "$createdAt"],
        "immutable": ["author"],
        "immutableAfter": { "text": 300 },
        "additionalProperties": false
    })
}

/// The base schema with one doctype-level key set (added or replaced).
fn post_schema_with(key: &str, value: Value) -> Value {
    let mut schema = post_schema();
    schema.set_value(key, value).expect("doctype key applies");
    schema
}

fn windows(entries: &[(&str, u32)]) -> BTreeMap<String, u32> {
    entries
        .iter()
        .map(|(property, seconds)| (property.to_string(), *seconds))
        .collect()
}

// ── the happy path ──────────────────────────────────────────────────────

#[test]
fn should_parse_the_windows_on_both_validation_modes() {
    let platform_version = PlatformVersion::latest();

    for full_validation in [false, true] {
        let document_type = parse_with(post_schema(), platform_version, full_validation)
            .unwrap_or_else(|error| {
                panic!("post schema should parse (full_validation: {full_validation}): {error}")
            });

        assert_eq!(
            document_type.immutable_after_seconds(),
            &windows(&[("text", 300)])
        );
    }
}

#[test]
fn should_freeze_nothing_later_when_the_keyword_is_omitted() {
    let mut schema = post_schema();
    let _ = schema.remove_optional_value("immutableAfter");

    let document_type = parse_with(schema, PlatformVersion::latest(), true).expect("schema parses");

    assert!(document_type.immutable_after_seconds().is_empty());
}

#[test]
fn should_accept_an_object_property_and_windows_of_any_length() {
    let schema = post_schema_with(
        "immutableAfter",
        platform_value!({ "text": 1, "meta": 4294967295u32 }),
    );

    let document_type = parse_with(schema, PlatformVersion::latest(), true)
        .expect("an object property and the widest window are accepted");

    assert_eq!(
        document_type.immutable_after_seconds(),
        &windows(&[("meta", u32::MAX), ("text", 1)])
    );
}

#[test]
fn should_parse_through_the_dispatcher_at_latest() {
    let document_type = parse_dispatched(post_schema(), PlatformVersion::latest(), true)
        .expect("post schema parses through the dispatcher at PV14");
    assert_eq!(
        document_type.as_ref().immutable_after_seconds(),
        &windows(&[("text", 300)])
    );
}

// ── the lints (full validation only) ────────────────────────────────────

#[test]
fn should_refuse_an_unknown_property() {
    let schema = post_schema_with("immutableAfter", platform_value!({ "nope": 60 }));
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), true),
        "\"nope\" under `immutableAfter`, but it is not a top-level property",
    );
}

#[test]
fn should_refuse_a_nested_path_with_a_hint() {
    let schema = post_schema_with("immutableAfter", platform_value!({ "meta.tag": 60 }));
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), true),
        "nested paths are not accepted",
    );
}

#[test]
fn should_refuse_a_system_property() {
    let schema = post_schema_with("immutableAfter", platform_value!({ "$ownerId": 60 }));
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), true),
        "system property \"$ownerId\"",
    );
}

#[test]
fn should_refuse_a_transient_property() {
    let schema = post_schema_with("transient", platform_value!(["text"]));
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), true),
        "\"text\" as both transient and under `immutableAfter`",
    );
}

/// A property frozen at creation has no window to give: the two keywords
/// would say different things of it.
#[test]
fn should_refuse_a_property_also_listed_under_immutable() {
    let schema = post_schema_with("immutableAfter", platform_value!({ "author": 60 }));
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), true),
        "\"author\" both under `immutable`",
    );
}

#[test]
fn should_refuse_the_windows_on_a_non_mutable_document_type() {
    let mut schema = post_schema_with("documentsMutable", Value::Bool(false));
    let _ = schema.remove_optional_value("immutable");
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), true),
        "lists `immutableAfter` properties but its documents are not mutable",
    );
}

/// The windows are measured from `$createdAt`, so every document must carry
/// it.
#[test]
fn should_refuse_the_windows_on_a_type_that_does_not_require_created_at() {
    let schema = post_schema_with("required", platform_value!(["author", "text"]));
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), true),
        "does not require `$createdAt`",
    );
}

// ── the shape (both validation modes) ───────────────────────────────────

#[test]
fn should_refuse_a_non_object_value_on_both_modes() {
    for full_validation in [false, true] {
        let schema = post_schema_with("immutableAfter", platform_value!(["text"]));
        expect_structure_error(
            parse_with(schema, PlatformVersion::latest(), full_validation),
            "`immutableAfter` must be an object mapping top-level property names to seconds",
        );
    }
}

/// A window of none is a property frozen at creation, which `immutable` says.
#[test]
fn should_refuse_a_window_of_zero_on_both_modes() {
    for full_validation in [false, true] {
        let schema = post_schema_with("immutableAfter", platform_value!({ "text": 0 }));
        expect_structure_error(
            parse_with(schema, PlatformVersion::latest(), full_validation),
            "an `immutableAfter` window of 0 seconds",
        );
    }
}

#[test]
fn should_refuse_a_window_that_is_not_whole_seconds_on_both_modes() {
    // A string, a fraction, a negative number and one above u32::MAX
    for window in [
        Value::Text("300".to_string()),
        Value::Float(1.5),
        Value::I64(-1),
        Value::U64(u64::from(u32::MAX) + 1),
    ] {
        for full_validation in [false, true] {
            let schema = post_schema_with(
                "immutableAfter",
                Value::Map(vec![(Value::Text("text".to_string()), window.clone())]),
            );
            expect_structure_error(
                parse_with(schema, PlatformVersion::latest(), full_validation),
                "must be a whole number of seconds from 1 to 4294967295",
            );
        }
    }
}

// ── the stored-contract path ────────────────────────────────────────────

/// The lints are validation-only: a stored contract is re-parsed without
/// validation and must come back exactly as declared.
#[test]
fn should_record_the_windows_as_declared_without_validation() {
    let schema = post_schema_with("immutableAfter", platform_value!({ "nope": 60 }));

    let document_type = parse_with(schema, PlatformVersion::latest(), false)
        .expect("the non-validating path records the declaration without judging it");

    assert_eq!(
        document_type.immutable_after_seconds(),
        &windows(&[("nope", 60)])
    );
}

// ── generation gating ───────────────────────────────────────────────────

#[test]
fn should_be_inert_below_generation_3_without_validation() {
    let platform_version_13 = PlatformVersion::get(13).expect("PV13 exists");
    let mut schema = post_schema();
    let _ = schema.remove_optional_value("immutable");

    let document_type = parse_dispatched(schema.clone(), platform_version_13, false)
        .expect("generation 2 ignores unknown doctype-level keywords when not validating");
    assert!(
        document_type.as_ref().immutable_after_seconds().is_empty(),
        "generation 2 must not record immutableAfter windows"
    );

    assert!(
        parse_dispatched(schema, platform_version_13, true).is_err(),
        "meta-schema v2 must reject the immutableAfter keyword"
    );
}

// ── references on a property frozen once its window passed ──────────────

/// Once its window passed, an `immutableAfter` property is as frozen as an
/// `immutable` one, so the reference rules judge them alike: a typed array of
/// `deletableDocument` references is refused (every replace re-validates the
/// elements and a frozen array could not drop a dead one), while a single one
/// by id held directly by the property is admitted, since a replace may clear
/// it once its target is deleted.
#[cfg(feature = "validation")]
mod deletable_document_reference {
    use super::*;

    fn identifier(refers_to: Value) -> Value {
        platform_value!({
            "type": "array",
            "byteArray": true,
            "minItems": 32,
            "maxItems": 32,
            "contentMediaType": "application/x.dash.dpp.identifier",
            "refersTo": refers_to
        })
    }

    fn post_schema_with_reference(reference: Value) -> Value {
        platform_value!({
            "type": "object",
            "documentsMutable": true,
            "properties": {
                "text": { "type": "string", "maxLength": 500, "position": 0 },
                "reply": reference
            },
            "required": ["text", "$createdAt"],
            "immutableAfter": { "text": 300, "reply": 300 },
            "additionalProperties": false
        })
    }

    fn deletable_draft() -> Value {
        platform_value!({ "type": "deletableDocument", "documentType": "draft" })
    }

    #[test]
    fn should_refuse_a_typed_array_of_deletable_document_references() {
        let mut replies = platform_value!({
            "type": "array",
            "minItems": 0,
            "maxItems": 8,
            "uniqueItems": true,
            "items": identifier(deletable_draft())
        });
        replies
            .set_value("position", Value::U32(1))
            .expect("position applies");

        expect_structure_error(
            parse_dispatched(
                post_schema_with_reference(replies),
                PlatformVersion::latest(),
                true,
            ),
            "lists \"reply\" under `immutableAfter`, but \"reply\" is a typed array of \
             deletableDocument references",
        );
    }

    #[test]
    fn should_admit_a_single_deletable_document_reference_by_id() {
        let mut reply = identifier(deletable_draft());
        reply
            .set_value("position", Value::U32(1))
            .expect("position applies");

        let document_type = parse_dispatched(
            post_schema_with_reference(reply),
            PlatformVersion::latest(),
            true,
        )
        .expect("a single deletableDocument reference by id may be frozen after a window");

        assert_eq!(
            document_type.as_ref().immutable_after_seconds(),
            &windows(&[("reply", 300), ("text", 300)])
        );
    }
}
