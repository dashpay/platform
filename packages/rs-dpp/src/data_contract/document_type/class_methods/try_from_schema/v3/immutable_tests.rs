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
//! An entry is a property name, frozen at creation, or `{ "property", "when" }`,
//! frozen while the condition holds. The rules on the listed properties
//! (`apply_immutable_fields`) are schema lints, so unlike the indexOnly matrix
//! they only run under `full_validation`; the non-validating path, which stored
//! contracts take, records the list as declared. The shape of the list and of
//! its entries, and what a condition reads, are enforced on both paths.

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
    parse_dispatched_with_defs(schema, None, platform_version, full_validation)
}

/// [`parse_dispatched`] with the contract's `$defs`, which a `$ref` in
/// `schema` resolves against.
pub(super) fn parse_dispatched_with_defs(
    schema: Value,
    schema_defs: Option<&BTreeMap<String, Value>>,
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
        schema_defs,
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

/// The lints surface as `InvalidContractStructure`, wrapped as the basic
/// `ContractError` whenever the `validation` feature is on: a bare
/// `ProtocolError::DataContractError` would refuse the transition unpaid.
pub(super) fn expect_structure_error<T: std::fmt::Debug>(
    result: Result<T, ProtocolError>,
    needle: &str,
) {
    let message = match result {
        #[cfg(not(feature = "validation"))]
        Err(ProtocolError::DataContractError(DataContractError::InvalidContractStructure(
            message,
        ))) => message,
        #[cfg(feature = "validation")]
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
            "must be an array of property names",
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

// ── conditional entries ─────────────────────────────────────────────────

/// Frozen five minutes after creation: `$updatedAt` is the replace's block
/// time on the document a replace writes.
fn five_minutes_after_creation() -> Value {
    platform_value!({
        "greaterThan": [{ "subtract": ["$updatedAt", "$createdAt"] }, 300000]
    })
}

/// `post_schema` recording `$createdAt` and `$updatedAt`, with `author`
/// frozen at creation, `body` five minutes after it and `meta` once the
/// stored document holds it.
fn post_schema_with_conditions() -> Value {
    let mut schema = post_schema_with(
        "immutable",
        platform_value!([
            "author",
            { "property": "body", "when": five_minutes_after_creation() },
            { "property": "meta", "when": { "present": "$old.meta" } }
        ]),
    );
    schema
        .set_value(
            "required",
            platform_value!(["author", "body", "$createdAt", "$updatedAt"]),
        )
        .expect("doctype key applies");
    schema
}

/// `post_schema_with_conditions` with `meta` frozen by `when` instead.
fn post_schema_with_meta_condition(when: Value) -> Value {
    let mut schema = post_schema_with_conditions();
    schema
        .set_value(
            "immutable",
            platform_value!(["author", { "property": "meta", "when": when }]),
        )
        .expect("doctype key applies");
    schema
}

#[test]
fn should_parse_conditional_entries_on_both_validation_modes() {
    for full_validation in [false, true] {
        let document_type = parse_with(
            post_schema_with_conditions(),
            PlatformVersion::latest(),
            full_validation,
        )
        .unwrap_or_else(|error| {
            panic!("the conditions should parse (full_validation: {full_validation}): {error}")
        });

        assert_eq!(document_type.immutable_fields(), &names(&["author"]));
        assert_eq!(
            document_type
                .immutable_field_conditions()
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            vec!["body", "meta"]
        );
    }
}

/// `immutableAllowSetting` said what `{ "present": "$old.<property>" }` says
/// now. Refused on every parse, naming its replacement, so that a contract
/// written with it never loads with another meaning.
#[test]
fn should_refuse_immutable_allow_setting_on_every_parse() {
    for full_validation in [false, true] {
        let schema = post_schema_with("immutableAllowSetting", platform_value!(["author"]));
        expect_structure_error(
            parse_with(schema, PlatformVersion::latest(), full_validation),
            "`immutableAllowSetting` is replaced by a conditional `immutable` entry",
        );
    }
}

#[test]
fn should_refuse_a_property_listed_twice_on_both_modes() {
    for immutable in [
        platform_value!(["author", "author"]),
        platform_value!(["author", { "property": "author", "when": { "present": "$old.author" } }]),
    ] {
        for full_validation in [false, true] {
            expect_structure_error(
                parse_with(
                    post_schema_with("immutable", immutable.clone()),
                    PlatformVersion::latest(),
                    full_validation,
                ),
                "lists \"author\" under `immutable` twice",
            );
        }
    }
}

#[test]
fn should_refuse_an_entry_object_of_another_shape_on_both_modes() {
    for (entry, needle) in [
        (
            platform_value!({ "property": "meta" }),
            "an `immutable` entry object needs \"property\"",
        ),
        (
            platform_value!({ "when": { "present": "meta" } }),
            "an `immutable` entry object needs \"property\"",
        ),
        (
            platform_value!({ "property": "meta", "when": { "present": "meta" }, "why": "x" }),
            "holds exactly \"property\" and \"when\", not \"why\"",
        ),
        (
            platform_value!({ "property": 7, "when": { "present": "meta" } }),
            "an `immutable` entry object needs \"property\"",
        ),
    ] {
        for full_validation in [false, true] {
            expect_structure_error(
                parse_with(
                    post_schema_with("immutable", Value::Array(vec![entry.clone()])),
                    PlatformVersion::latest(),
                    full_validation,
                ),
                needle,
            );
        }
    }
}

/// What a condition reads is checked on every parse, as a rule's reads are: a
/// stored condition reading a property the type does not declare could only
/// ever read it as absent.
#[test]
fn should_refuse_a_condition_reading_what_the_type_does_not_hold_on_both_modes() {
    for (when, needle) in [
        (
            platform_value!({ "present": "nope" }),
            "immutable condition of \"meta\" tests the presence of \"nope\", which is not a \
             property",
        ),
        (
            platform_value!({ "present": "$old.nope" }),
            "immutable condition of \"meta\" tests the presence of \"$old.nope\", which is not a \
             property",
        ),
        (
            platform_value!({ "greaterThan": ["$old.author", 1] }),
            "immutable condition of \"meta\" reads \"$old.author\", which has type string",
        ),
        (
            platform_value!({ "greaterThan": ["$transferredAt", 1] }),
            "immutable condition of \"meta\" reads $transferredAt, which the document type does \
             not record",
        ),
        (
            platform_value!({ "bogus": 1 }),
            "immutable condition of \"meta\" names \"bogus\", which is not a comparison",
        ),
    ] {
        for full_validation in [false, true] {
            expect_structure_error(
                parse_with(
                    post_schema_with_meta_condition(when.clone()),
                    PlatformVersion::latest(),
                    full_validation,
                ),
                needle,
            );
        }
    }
}

/// A condition reads the document alone, so a replace judges it without
/// reading state.
#[test]
fn should_refuse_a_condition_reading_a_total_on_both_modes() {
    for full_validation in [false, true] {
        expect_structure_error(
            parse_with(
                post_schema_with_meta_condition(
                    platform_value!({ "greaterThan": [{ "countOf": ["post"] }, 0] }),
                ),
                PlatformVersion::latest(),
                full_validation,
            ),
            "immutable condition of \"meta\" reads a countOf or sumOf total",
        );
    }
}

/// `$old.` is the stored document, which only a replace has: a
/// `propertyConstraints` rule judges a create too.
#[test]
fn should_refuse_a_stored_read_in_a_property_constraints_rule() {
    let schema = post_schema_with(
        "propertyConstraints",
        platform_value!({ "keepsMeta": { "present": "$old.meta" } }),
    );
    // The meta-schema's path pattern refuses it first under full validation;
    // the parse refuses it on the stored path as well
    assert!(parse_with(schema.clone(), PlatformVersion::latest(), true).is_err());
    expect_structure_error(
        parse_with(schema, PlatformVersion::latest(), false),
        "rule \"keepsMeta\" reads \"$old.meta\", but only a condition judging a replace (an \
         `immutable` entry's, or `retractedWhen`) reads the stored document",
    );
}

/// The lints are validation-only, as for the entries without a condition.
#[test]
fn should_record_a_condition_as_declared_without_the_lints() {
    let mut schema = post_schema_with_conditions();
    schema
        .set_value("documentsMutable", Value::Bool(false))
        .expect("doctype key applies");

    expect_structure_error(
        parse_with(schema.clone(), PlatformVersion::latest(), true),
        "documentsMutable: false",
    );
    let document_type = parse_with(schema, PlatformVersion::latest(), false)
        .expect("the non-validating path records the declaration without the lints");
    assert_eq!(document_type.immutable_field_conditions().len(), 2);
}

#[test]
fn should_ignore_conditional_entries_below_generation_3_without_validation() {
    let platform_version_13 = PlatformVersion::get(13).expect("PV13 exists");

    let document_type = parse_dispatched(post_schema_with_conditions(), platform_version_13, false)
        .expect("generation 2 ignores unknown doctype-level keywords when not validating");
    assert!(document_type.immutable_field_conditions().is_empty());

    assert!(
        parse_dispatched(post_schema_with_conditions(), platform_version_13, true).is_err(),
        "meta-schema v2 must reject the immutable keyword"
    );
}

// ── conditional entries on a deletableDocument reference ────────────────

/// A replace may clear a `deletableDocument` reference held by an immutable
/// top-level property once its target is deleted, since every replace
/// re-validates it. Frozen only while a condition holds, the property would
/// take another value from a replace the condition leaves free (`present:
/// "$old.pinnedId"` after the clear), and the frozen reference would point at
/// another document, so generation 3 refuses the pair under full validation.
/// The refusal sits with the other immutable `deletableDocument` refusals,
/// which are `validation` feature code, as `validate_update` is.
#[cfg(feature = "validation")]
mod deletable_document_reference {
    use super::*;
    use crate::block::block_info::BlockInfo;
    use crate::data_contract::methods::validate_update::DataContractUpdateValidationMethodsV0;
    use crate::data_contract::serialized_version::v0::DataContractInSerializationFormatV0;
    use crate::data_contract::serialized_version::DataContractInSerializationFormat;
    use crate::data_contract::DataContract;

    const REFUSAL: &str = "lists \"pinnedId\" under `immutable` with a condition, but it is a \
                           deletableDocument reference";

    fn deletable_draft() -> Value {
        platform_value!({ "type": "deletableDocument", "documentType": "draft" })
    }

    fn pinned_once() -> Value {
        platform_value!({ "property": "pinnedId", "when": { "present": "$old.pinnedId" } })
    }

    /// A mutable `post` whose optional top-level `pinnedId` refers to another
    /// document by id through `refers_to`, with the given `immutable` list.
    fn post_schema_with_pinned_reference(refers_to: Value, immutable: Value) -> Value {
        platform_value!({
            "type": "object",
            "documentsMutable": true,
            "properties": {
                "author": { "type": "string", "maxLength": 63, "position": 0 },
                "pinnedId": {
                    "type": "array",
                    "byteArray": true,
                    "minItems": 32,
                    "maxItems": 32,
                    "contentMediaType": "application/x.dash.dpp.identifier",
                    "refersTo": refers_to,
                    "position": 1
                }
            },
            "required": ["author"],
            "immutable": immutable,
            "additionalProperties": false
        })
    }

    /// The sequence this closes: `pinnedId` set to draft A, A deleted, a
    /// replace clears `pinnedId` (a dead immutable reference may be cleared),
    /// and the next replace sets it to draft B, which the condition allows as
    /// the stored document no longer holds it. Refused as a consensus error,
    /// so a registration or update carrying it fails deterministically.
    #[test]
    fn should_refuse_a_condition_on_a_deletable_document_reference() {
        let schema = post_schema_with_pinned_reference(
            deletable_draft(),
            Value::Array(vec![Value::Text("author".to_string()), pinned_once()]),
        );

        match parse_dispatched(schema.clone(), PlatformVersion::latest(), true) {
            Err(ProtocolError::ConsensusError(error)) => match *error {
                ConsensusError::BasicError(BasicError::ContractError(
                    DataContractError::InvalidContractStructure(message),
                )) => assert!(
                    message.contains(REFUSAL),
                    "expected {REFUSAL:?} in the error, got: {message}"
                ),
                other => panic!("expected InvalidContractStructure, got {other:?}"),
            },
            other => panic!("expected a consensus error, got {other:?}"),
        }

        // A registration rule: a stored contract is re-parsed without it and
        // stays readable
        let stored = parse_dispatched(schema, PlatformVersion::latest(), false)
            .expect("the non-validating path records the declaration without judging it");
        assert!(stored
            .as_ref()
            .immutable_field_conditions()
            .contains_key("pinnedId"));
    }

    /// Without a condition the reference is frozen at creation, and the clear
    /// is its one way out once its target is deleted.
    #[test]
    fn should_admit_an_immutable_deletable_document_reference_without_a_condition() {
        let document_type = parse_dispatched(
            post_schema_with_pinned_reference(
                deletable_draft(),
                platform_value!(["author", "pinnedId"]),
            ),
            PlatformVersion::latest(),
            true,
        )
        .expect("an immutable top-level deletableDocument reference registers");

        assert_eq!(
            document_type.immutable_fields(),
            &names(&["author", "pinnedId"])
        );
        assert!(document_type.immutable_field_conditions().is_empty());
    }

    /// The clear exists for a `deletableDocument` reference by id only. A
    /// permanent document and an identity are never deleted, so a reference
    /// to either is never cleared and may be frozen by a condition.
    #[test]
    fn should_admit_a_condition_on_a_reference_that_is_never_cleared() {
        for refers_to in [
            platform_value!({ "type": "permanentDocument", "documentType": "article" }),
            platform_value!({ "type": "identity" }),
        ] {
            let document_type = parse_dispatched(
                post_schema_with_pinned_reference(
                    refers_to.clone(),
                    Value::Array(vec![Value::Text("author".to_string()), pinned_once()]),
                ),
                PlatformVersion::latest(),
                true,
            )
            .unwrap_or_else(|error| {
                panic!("{refers_to:?} frozen by a condition should register: {error}")
            });

            assert!(document_type
                .as_ref()
                .immutable_field_conditions()
                .contains_key("pinnedId"));
        }
    }

    /// A contract at `version` with a deletable `draft` type and a `post`
    /// whose `pinnedId` refers to a draft by id, the post's `immutable` list
    /// as given.
    fn pinned_contract(
        version: u32,
        immutable: Value,
        platform_version: &PlatformVersion,
    ) -> DataContractInSerializationFormat {
        let config = DataContractConfig::default_for_version(platform_version)
            .expect("default config available on this platform version");
        let draft = platform_value!({
            "type": "object",
            "properties": {
                "body": { "type": "string", "maxLength": 500, "position": 0 }
            },
            "additionalProperties": false
        });

        DataContractInSerializationFormatV0 {
            id: Identifier::new([7; 32]),
            config,
            version,
            owner_id: Identifier::new([8; 32]),
            schema_defs: None,
            document_schemas: BTreeMap::from([
                ("draft".to_string(), draft),
                (
                    "post".to_string(),
                    post_schema_with_pinned_reference(deletable_draft(), immutable),
                ),
            ]),
        }
        .into()
    }

    /// `validate_update` 1 lets a property the list did not hold arrive with a
    /// condition, so an update can reach the pair too. The update transition
    /// parses the whole new contract under full validation, as a registration
    /// does, and that parse refuses it.
    #[test]
    fn should_refuse_an_update_freezing_a_deletable_document_reference_by_a_condition() {
        let platform_version = PlatformVersion::latest();

        let registered = DataContract::try_from_platform_versioned(
            pinned_contract(1, platform_value!(["author"]), platform_version),
            true,
            &mut vec![],
            platform_version,
        )
        .expect("the contract registers with a mutable deletableDocument reference");

        let update = pinned_contract(
            2,
            Value::Array(vec![Value::Text("author".to_string()), pinned_once()]),
            platform_version,
        );

        // The update rules alone admit it (parsed without the registration
        // rules to get that far), so the parse is what stops it
        let unchecked = DataContract::try_from_platform_versioned(
            update.clone(),
            false,
            &mut vec![],
            platform_version,
        )
        .expect("the non-validating parse records the list as declared");
        let result = registered
            .validate_update(&unchecked, &BlockInfo::default(), platform_version)
            .expect("the update is judged");
        assert!(
            result.is_valid(),
            "a property the list did not hold may arrive with a condition: {:?}",
            result.errors
        );

        expect_structure_error(
            DataContract::try_from_platform_versioned(update, true, &mut vec![], platform_version),
            REFUSAL,
        );
    }
}
