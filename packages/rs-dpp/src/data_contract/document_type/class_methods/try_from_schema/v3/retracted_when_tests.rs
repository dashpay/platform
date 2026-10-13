//! The `retractedWhen` doctype keyword (protocol version 14): the condition under which a replace
//! writes a retracted document, which a banned or suspended owner may still make.
use super::immutable_tests::expect_structure_error;
use super::*;
use crate::data_contract::config::moderation::{ContractModerationConfig, ContractModerators};
use crate::data_contract::document_type::accessors::DocumentTypeV2Getters;
use crate::data_contract::document_type::property_constraints::{
    DocumentSystemValues, STORED_DOCUMENT_KEY,
};
use platform_value::platform_value;

/// A contract config keeping the lists named, moderated by the contract owner.
fn config_keeping(banlist: bool, suspensions: bool, warnings: bool) -> DataContractConfig {
    DataContractConfig::default_for_version(PlatformVersion::latest())
        .expect("default config available")
        .with_moderation(Some(ContractModerationConfig {
            banlist,
            suspensions,
            moderators: ContractModerators::ContractOwner,
            warnings,
        }))
}

fn parse_with_config(
    schema: Value,
    config: &DataContractConfig,
    full_validation: bool,
) -> Result<DocumentType, ProtocolError> {
    let platform_version = PlatformVersion::latest();
    DocumentType::try_from_schema(
        Identifier::new([1; 32]),
        1,
        config.version(),
        "post",
        schema,
        None,
        &BTreeMap::new(),
        config,
        full_validation,
        &mut vec![],
        platform_version,
    )
}

/// A `post` retracted by setting `deleted`, with `extra` merged in at the top level.
fn post_schema(retracted_when: Value, extra: Value) -> Value {
    let mut schema = platform_value!({
        "type": "object",
        "properties": {
            "text": { "type": "string", "maxLength": 50, "position": 0 },
            "deleted": { "type": "boolean", "position": 1 },
        },
        "additionalProperties": false,
        "canBeDeleted": false,
        "retractedWhen": retracted_when,
    });
    if let (Value::Map(schema_map), Value::Map(extra_map)) = (&mut schema, extra) {
        for (key, value) in extra_map {
            schema_map.retain(|(existing, _)| existing != &key);
            schema_map.push((key, value));
        }
    }
    schema
}

fn deleted_post_schema() -> Value {
    post_schema(
        platform_value!({ "present": "deleted" }),
        platform_value!({}),
    )
}

/// Whether the parsed condition holds for `data` written over `stored`.
fn holds(document_type: &DocumentType, data: Value, stored: Value) -> bool {
    let Value::Map(mut entries) = data else {
        panic!("expected a map");
    };
    entries.push((Value::Text(STORED_DOCUMENT_KEY.to_string()), stored));
    document_type
        .retracted_when()
        .expect("expected a retractedWhen condition")
        .holds(&Value::Map(entries), &DocumentSystemValues::default())
        .expect("expected the condition to evaluate")
}

#[test]
fn should_parse_the_condition_on_a_mutable_type_of_a_contract_keeping_a_bar() {
    for (banlist, suspensions) in [(true, false), (false, true), (true, true)] {
        for full_validation in [false, true] {
            let document_type = parse_with_config(
                deleted_post_schema(),
                &config_keeping(banlist, suspensions, false),
                full_validation,
            )
            .expect("expected the document type to parse");
            assert!(holds(
                &document_type,
                platform_value!({ "deleted": true }),
                platform_value!({ "text": "hello" }),
            ));
            assert!(!holds(
                &document_type,
                platform_value!({ "text": "edited" }),
                platform_value!({ "text": "hello" }),
            ));
        }
    }
}

#[test]
fn should_have_no_condition_without_the_keyword() {
    let mut schema = deleted_post_schema();
    if let Value::Map(map) = &mut schema {
        map.retain(|(key, _)| key != &Value::Text("retractedWhen".to_string()));
    }
    let document_type = parse_with_config(schema, &config_keeping(true, false, false), true)
        .expect("expected the document type to parse");
    assert!(document_type.retracted_when().is_none());
}

/// Like an `immutable` condition, it may read the stored document: a type whose retracted
/// documents stay retracted can say so in its own `immutable` list, and the condition may still
/// tell a first retraction from a later replace.
#[test]
fn should_parse_a_condition_reading_the_stored_document() {
    let document_type = parse_with_config(
        post_schema(
            platform_value!({ "allOf": [{ "present": "deleted" }, { "absent": "$old.deleted" }] }),
            platform_value!({}),
        ),
        &config_keeping(true, false, false),
        true,
    )
    .expect("expected the document type to parse");
    assert!(holds(
        &document_type,
        platform_value!({ "deleted": true }),
        platform_value!({ "text": "hello" }),
    ));
    assert!(!holds(
        &document_type,
        platform_value!({ "deleted": true }),
        platform_value!({ "deleted": true }),
    ));
}

/// Without a replace there is nothing to retract with.
#[test]
fn should_refuse_the_keyword_on_a_type_whose_documents_are_not_mutable() {
    for full_validation in [false, true] {
        expect_structure_error(
            parse_with_config(
                post_schema(
                    platform_value!({ "present": "deleted" }),
                    platform_value!({ "documentsMutable": false }),
                ),
                &config_keeping(true, false, false),
                full_validation,
            ),
            "document type \"post\" sets `retractedWhen`, but its documents are not mutable",
        );
    }
}

/// A contract that bars nobody, unmoderated or keeping only a warning list, has no owner the
/// keyword could let through.
#[test]
fn should_refuse_the_keyword_on_a_contract_that_bars_nobody() {
    let unmoderated =
        DataContractConfig::default_for_version(PlatformVersion::latest()).expect("config");
    for config in [unmoderated, config_keeping(false, false, true)] {
        for full_validation in [false, true] {
            expect_structure_error(
                parse_with_config(deleted_post_schema(), &config, full_validation),
                "the contract keeps neither a banlist nor a suspension list",
            );
        }
    }
}

#[test]
fn should_refuse_a_condition_that_does_not_parse_or_reads_what_the_type_lacks() {
    for (condition, needle) in [
        (
            platform_value!({ "present": "nope" }),
            "document type \"post\" retractedWhen condition tests the presence of \"nope\"",
        ),
        (
            platform_value!({ "present": "$old.nope" }),
            "document type \"post\" retractedWhen condition tests the presence of \"$old.nope\"",
        ),
        (
            platform_value!({ "bogus": 1 }),
            "document type \"post\" retractedWhen condition names \"bogus\", which is not a \
             comparison",
        ),
        (
            platform_value!({ "greaterThan": [{ "countOf": ["post"] }, 0] }),
            "document type \"post\" retractedWhen condition reads a countOf or sumOf total",
        ),
    ] {
        for full_validation in [false, true] {
            expect_structure_error(
                parse_with_config(
                    post_schema(condition.clone(), platform_value!({})),
                    &config_keeping(true, false, false),
                    full_validation,
                ),
                needle,
            );
        }
    }
}

/// The meta-schema takes a single condition, as it does an `immutable` entry's `when`; a stored
/// contract the meta-schema is not run on is refused by the condition parse instead.
#[test]
fn should_refuse_a_value_that_is_not_one_condition() {
    for value in [
        platform_value!(true),
        platform_value!({}),
        platform_value!({ "present": "deleted", "absent": "text" }),
    ] {
        for full_validation in [false, true] {
            assert!(
                parse_with_config(
                    post_schema(value.clone(), platform_value!({})),
                    &config_keeping(true, false, false),
                    full_validation,
                )
                .is_err(),
                "expected {value:?} to be refused (full validation {full_validation})"
            );
        }
    }
}

/// What a barred owner may still write is fixed with the type: an update may not add the
/// condition, remove it or change it, and keeping it as it is passes.
#[test]
fn should_keep_the_condition_fixed_on_contract_update() {
    use crate::consensus::state::state_error::StateError;
    use crate::consensus::ConsensusError;

    let platform_version = PlatformVersion::latest();
    let config = config_keeping(true, false, false);
    let parse = |schema: Value| {
        parse_with_config(schema, &config, true).expect("expected the document type to parse")
    };
    let mut without = deleted_post_schema();
    if let Value::Map(map) = &mut without {
        map.retain(|(key, _)| key != &Value::Text("retractedWhen".to_string()));
    }
    let deleted = parse(deleted_post_schema());
    let blanked = parse(post_schema(
        platform_value!({ "absent": "text" }),
        platform_value!({}),
    ));
    let none = parse(without);

    let unchanged = deleted
        .as_ref()
        .validate_update(deleted.as_ref(), 2, platform_version)
        .expect("validate_update should not error");
    assert!(unchanged.is_valid(), "{:?}", unchanged.errors);

    for (old, new, change) in [
        (&none, &deleted, "add"),
        (&deleted, &none, "remove"),
        (&deleted, &blanked, "change"),
    ] {
        let result = old
            .as_ref()
            .validate_update(new.as_ref(), 2, platform_version)
            .expect("validate_update should not error");
        let expected = format!("document type can not {change} its `retractedWhen` condition");
        assert!(
            result.errors.iter().any(|error| matches!(
                error,
                ConsensusError::StateError(StateError::DocumentTypeUpdateError(e))
                    if e.additional_message().contains(&expected)
            )),
            "expected {expected:?}, got: {:?}",
            result.errors
        );
    }
}
