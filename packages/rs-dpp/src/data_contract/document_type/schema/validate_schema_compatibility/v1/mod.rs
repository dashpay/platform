//! Protocol v14 generation of the JSON-schema compatibility check.
//!
//! The compatibility validator walks the JSON diff between the old and new
//! document type schemas and hard-errors (`UnsupportedSchemaKeywordError`,
//! surfaced as an internal error rather than a consensus-invalid result) on
//! any keyword it has no rule for — and it has no rule for `indices`. Index
//! changes are not this check's concern: `validate_update` v1 compares the
//! parsed index definitions by name and rejects any added, removed or
//! modified index with a clean consensus error before schema compatibility
//! runs. The only `/indices` diff that could survive to this check is a
//! reordering of the array that leaves the definition set identical — a
//! semantic no-op (indices are keyed by name) that under v0 still hit the
//! hard error.
//!
//! v1 therefore strips the top-level `indices` key from both schemas before
//! diffing, so index definitions are validated in exactly one place. Only
//! the document type's own `indices` keyword is removed; a *property* named
//! `indices` lives under `/properties/indices` and is still validated.
//!
//! The top-level `required` key is stripped for the same reason: top-level
//! requiredness changes are judged by `validate_update` v1's
//! `validate_required_fields_update`, which admits exactly one change the
//! differ's frozen `required` rule cannot express — a brand-new property
//! added as required with `requiredSince` equal to the version the update
//! creates. Nested `required` arrays (under `/properties/<name>/required`)
//! remain frozen by the differ.
//!
//! The top-level `immutable` key (protocol version 14) is stripped as well:
//! the differ has no rule for it, and `validate_update` v1's
//! `validate_immutable_fields_update` judges it (the list may grow, never
//! shrink).
//!
//! The top-level `ownerRefersTo` and `creatorRefersTo` keys (protocol version
//! 14) get the frozen rule of the property `refersTo`, so any change to them
//! is an incompatible schema change. So does the top-level `transient` list,
//! which generation 0 fails on as an unsupported keyword, compared as the set
//! of names the parse reads: sorted and deduplicated before the diff. And so
//! does the top-level `propertyConstraints` object (protocol version 14):
//! every stored document was judged against the rules it names, so none may be
//! added, removed or changed.

use crate::data_contract::document_type::property_names::{PROPERTY_CONSTRAINTS, TRANSIENT};
use crate::data_contract::document_type::schema::IncompatibleJsonSchemaOperation;
use crate::data_contract::errors::{DataContractError, JsonSchemaError};
use crate::data_contract::JsonValue;
use crate::validation::SimpleValidationResult;
use crate::ProtocolError;
use json_schema_compatibility_validator::{
    validate_schemas_compatibility, CompatibilityRulesCollection, Options,
    KEYWORD_COMPATIBILITY_RULES,
};
use once_cell::sync::Lazy;
use std::borrow::Cow;
use std::ops::Deref;

static OPTIONS: Lazy<Options> = Lazy::new(|| {
    let mut required_rule = KEYWORD_COMPATIBILITY_RULES
        .get("required")
        .expect("required rule must be present")
        .clone();

    required_rule.allow_removal = false;
    required_rule
        .inner
        .as_mut()
        .expect("required rule must have inner rules")
        .allow_removal = false;

    // `ownerRefersTo` and `creatorRefersTo` (protocol version 14) are the
    // document type's own `refersTo`, whose value is the writer or the
    // creator: frozen exactly as the property keyword is, so adding, removing
    // or changing one is reported as an incompatible change. The rules live
    // here rather than in the shared rule set, which the generation-0 check
    // also reads, because no earlier protocol version knows the keywords.
    // Without a `refersTo` rule to copy, a diff under either fails as an
    // unsupported keyword, an error rather than a panic.
    // The top-level `transient` list gets the same frozen rule: it decides
    // which values a stored document carries and how each property is encoded
    // (a transient one takes a presence byte even when required), so documents
    // written under one list could not be read under another. Without a rule
    // the differ fails on any change to it as an unsupported keyword, an
    // internal error instead of an incompatible schema change. The parse reads
    // the list as a set, so it is sorted and deduplicated before the diff
    // ([`prepared_for_diff`]): only a changed set of names is a change.
    // The top-level `propertyConstraints` gets it too: a rule added later
    // would judge replaces of documents stored without it, and a rule changed
    // or removed would leave stored documents judged by one no longer there.
    let refers_to_rule = KEYWORD_COMPATIBILITY_RULES.get("refersTo");
    let frozen_doctype_rules = [
        "ownerRefersTo",
        "creatorRefersTo",
        TRANSIENT,
        PROPERTY_CONSTRAINTS,
    ]
    .into_iter()
    .filter_map(|keyword| refers_to_rule.map(|rule| (keyword, rule.clone())));

    Options {
        override_rules: CompatibilityRulesCollection::from_iter(
            [("required", required_rule)]
                .into_iter()
                .chain(frozen_doctype_rules),
        ),
    }
});

/// The document type's own top-level keys whose changes are validated by
/// dedicated checks in `validate_update` v1 instead of the JSON diff:
/// `indices` (index definitions compared by name), `required`
/// (`validate_required_fields_update`, which admits new-property additions
/// annotated with `requiredSince`), and `immutable` together with
/// `immutableAllowSetting` (`validate_immutable_fields_update`: the first may
/// only grow, the second may only shrink except for newly immutable
/// properties). The differ has no rule for the last three at all and would
/// hard-error on any change to them.
const TOP_LEVEL_VALIDATED_KEYS: [&str; 4] =
    ["indices", "required", "immutable", "immutableAllowSetting"];

/// Prepares a document type schema to be diffed: strips
/// [`TOP_LEVEL_VALIDATED_KEYS`], and sorts and deduplicates the top-level
/// `transient` list, which the parse reads as a set, so reordering or
/// repeating names is no change. Only the document type's own top-level keys
/// are touched; a nested object property's `required` array lives under
/// `/properties/<name>/required` and stays governed by the differ's frozen
/// `required` rule, as do properties named `indices`, `required`,
/// `immutable` or `transient`.
fn prepared_for_diff(schema: &JsonValue) -> Cow<'_, JsonValue> {
    match schema {
        JsonValue::Object(map)
            if map.contains_key(TRANSIENT)
                || TOP_LEVEL_VALIDATED_KEYS
                    .iter()
                    .any(|key| map.contains_key(*key)) =>
        {
            let mut map = map.clone();
            for key in TOP_LEVEL_VALIDATED_KEYS {
                map.remove(key);
            }
            if let Some(JsonValue::Array(names)) = map.get_mut(TRANSIENT) {
                names.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
                names.dedup();
            }
            Cow::Owned(JsonValue::Object(map))
        }
        _ => Cow::Borrowed(schema),
    }
}

/// Pairing invariant: stripping `indices`, top-level `required` and
/// `immutable` unconditionally is only safe because every `PlatformVersion`
/// that selects this generation (`validate_schema_compatibility: 1`) also
/// selects a `validate_update` generation of at least 1
/// (`dpp.validation.document_type.validate_update`), which rejects every
/// real index change, every disallowed required-set change and every
/// shrinking of the immutable list before this check runs. A future version
/// table that bumps one without the other would let those changes bypass
/// compatibility validation entirely.
pub(super) fn validate_schema_compatibility_v1(
    original_schema: &JsonValue,
    new_schema: &JsonValue,
) -> Result<SimpleValidationResult<IncompatibleJsonSchemaOperation>, ProtocolError> {
    let original_schema = prepared_for_diff(original_schema);
    let new_schema = prepared_for_diff(new_schema);

    validate_schemas_compatibility(&original_schema, &new_schema, OPTIONS.deref())
        .map(|result| {
            let errors = result
                .into_changes()
                .into_iter()
                .map(|change| IncompatibleJsonSchemaOperation {
                    name: change.name().to_string(),
                    path: change.path().to_string(),
                })
                .collect::<Vec<_>>();

            SimpleValidationResult::new_with_errors(errors)
        })
        .map_err(|error| {
            ProtocolError::DataContractError(DataContractError::JsonSchema(
                JsonSchemaError::SchemaCompatibilityValidationError(error.to_string()),
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::super::validate_schema_compatibility;
    use crate::data_contract::errors::{DataContractError, JsonSchemaError};
    use crate::ProtocolError;
    use assert_matches::assert_matches;
    use platform_version::version::PlatformVersion;
    use serde_json::json;

    #[test]
    fn should_ignore_indices_reordering() {
        let platform_version = PlatformVersion::latest();

        let original_schema = json!({
            "type": "object",
            "properties": {
                "a": {"type": "string", "position": 0},
                "b": {"type": "string", "position": 1},
            },
            "indices": [
                {"name": "j", "properties": [{"a": "asc"}]},
                {"name": "k", "properties": [{"b": "asc"}]},
            ],
            "additionalProperties": false,
        });

        let new_schema = json!({
            "type": "object",
            "properties": {
                "a": {"type": "string", "position": 0},
                "b": {"type": "string", "position": 1},
            },
            "indices": [
                {"name": "k", "properties": [{"b": "asc"}]},
                {"name": "j", "properties": [{"a": "asc"}]},
            ],
            "additionalProperties": false,
        });

        let result = validate_schema_compatibility(&original_schema, &new_schema, platform_version)
            .expect("an indices-only diff must not error");

        assert!(
            result.is_valid(),
            "an indices-only diff must be ignored, got {:?}",
            result.errors
        );
    }

    // The differ has no rule for `immutable`; a change to the list is judged
    // by `validate_update` v1 and must be invisible here rather than a
    // hard error.
    #[test]
    fn should_ignore_immutable_list_change() {
        let platform_version = PlatformVersion::latest();

        let original_schema = json!({
            "type": "object",
            "properties": {
                "a": {"type": "string", "position": 0},
                "b": {"type": "string", "position": 1},
            },
            "immutable": ["a"],
            "additionalProperties": false,
        });

        let new_schema = json!({
            "type": "object",
            "properties": {
                "a": {"type": "string", "position": 0},
                "b": {"type": "string", "position": 1},
            },
            "immutable": ["a", "b"],
            "immutableAllowSetting": ["b"],
            "additionalProperties": false,
        });

        let result = validate_schema_compatibility(&original_schema, &new_schema, platform_version)
            .expect("an immutable-only diff must not error");

        assert!(
            result.is_valid(),
            "an immutable-only diff must be ignored, got {:?}",
            result.errors
        );
    }

    // Stripping `indices` must not mask incompatible changes elsewhere in
    // the schema.
    #[test]
    fn should_still_report_incompatible_property_change_alongside_indices_diff() {
        let platform_version = PlatformVersion::latest();

        let original_schema = json!({
            "type": "object",
            "properties": {
                "a": {"type": "string", "position": 0},
            },
            "indices": [
                {"name": "j", "properties": [{"a": "asc"}]},
            ],
            "additionalProperties": false,
        });

        let new_schema = json!({
            "type": "object",
            "properties": {
                "a": {"type": "number", "position": 0},
            },
            "indices": [
                {"name": "k", "properties": [{"a": "asc"}]},
            ],
            "additionalProperties": false,
        });

        let result = validate_schema_compatibility(&original_schema, &new_schema, platform_version)
            .expect("schema compatibility validation should not error");

        assert_matches!(
            result.errors.as_slice(),
            [change] if change.name == "replace" && change.path == "/properties/a/type"
        );
    }

    // Only the document type's own top-level `indices` keyword is stripped;
    // a property that happens to be named "indices" sits under
    // `/properties/indices` and must still be validated.
    #[test]
    fn should_still_validate_property_named_indices() {
        let platform_version = PlatformVersion::latest();

        let original_schema = json!({
            "type": "object",
            "properties": {
                "indices": {"type": "string", "position": 0},
            },
            "additionalProperties": false,
        });

        let new_schema = json!({
            "type": "object",
            "properties": {
                "indices": {"type": "number", "position": 0},
            },
            "additionalProperties": false,
        });

        let result = validate_schema_compatibility(&original_schema, &new_schema, platform_version)
            .expect("schema compatibility validation should not error");

        assert_matches!(
            result.errors.as_slice(),
            [change] if change.name == "replace" && change.path == "/properties/indices/type"
        );
    }

    // Replay-safety pin: protocol version 13 dispatches to v0, where an
    // `/indices` diff still hits the unsupported-keyword hard error.
    #[test]
    fn v0_should_error_on_indices_diff() {
        let platform_version = PlatformVersion::get(13).expect("protocol version 13 must exist");

        let original_schema = json!({
            "type": "object",
            "properties": {
                "a": {"type": "string", "position": 0},
            },
            "indices": [
                {"name": "j", "properties": [{"a": "asc"}]},
            ],
            "additionalProperties": false,
        });

        let new_schema = json!({
            "type": "object",
            "properties": {
                "a": {"type": "string", "position": 0},
            },
            "indices": [
                {"name": "j", "properties": [{"a": "asc"}], "unique": true},
            ],
            "additionalProperties": false,
        });

        let error = validate_schema_compatibility(&original_schema, &new_schema, platform_version)
            .expect_err("an indices diff must hard-error under v0");

        assert_matches!(
            error,
            ProtocolError::DataContractError(DataContractError::JsonSchema(
                JsonSchemaError::SchemaCompatibilityValidationError(message)
            )) if message == "schema keyword 'indices' at path '/indices/0/unique' is not supported"
        );
    }

    fn with_transient(transient: Option<serde_json::Value>) -> serde_json::Value {
        let mut schema = json!({
            "type": "object",
            "properties": {
                "a": {"type": "string", "position": 0},
                "b": {"type": "string", "position": 1},
            },
            "additionalProperties": false,
        });
        if let Some(transient) = transient {
            schema["transient"] = transient;
        }
        schema
    }

    /// Stored documents are encoded by the transient list (a transient
    /// property takes a presence byte), so no change to it is compatible.
    #[test]
    fn should_report_every_transient_list_change_as_incompatible() {
        let platform_version = PlatformVersion::latest();
        for (original, new, change_name, change_path) in [
            (None, Some(json!(["a"])), "add", "/transient"),
            (Some(json!(["a"])), None, "remove", "/transient"),
            (
                Some(json!(["a"])),
                Some(json!(["a", "b"])),
                "add",
                "/transient/1",
            ),
            (
                Some(json!(["a"])),
                Some(json!(["b"])),
                "replace",
                "/transient/0",
            ),
        ] {
            let result = validate_schema_compatibility(
                &with_transient(original.clone()),
                &with_transient(new.clone()),
                platform_version,
            )
            .expect("a transient change is judged, not an unsupported keyword");
            assert_matches!(
                result.errors.as_slice(),
                [change] if change.name == change_name && change.path == change_path,
                "{original:?} -> {new:?}"
            );
        }

        // An unchanged list is no change at all
        let unchanged = with_transient(Some(json!(["a"])));
        assert!(
            validate_schema_compatibility(&unchanged, &unchanged, platform_version)
                .expect("an unchanged schema is judged")
                .is_valid()
        );
    }

    /// The parse reads the list as a set, so reordering or repeating names
    /// changes nothing a stored document depends on.
    #[test]
    fn should_accept_a_reordered_or_repeated_transient_list() {
        let platform_version = PlatformVersion::latest();
        for new in [json!(["b", "a"]), json!(["a", "b", "a"])] {
            let result = validate_schema_compatibility(
                &with_transient(Some(json!(["a", "b"]))),
                &with_transient(Some(new.clone())),
                platform_version,
            )
            .expect("a transient change is judged, not an unsupported keyword");
            assert!(result.is_valid(), "{new:?}: {:?}", result.errors);
        }
    }

    // Replay-safety pin: protocol version 13 dispatches to v0, where a
    // `/transient` diff still hits the unsupported-keyword hard error.
    #[test]
    fn should_hard_error_on_a_transient_diff_at_protocol_version_13() {
        let platform_version = PlatformVersion::get(13).expect("protocol version 13 must exist");
        let error = validate_schema_compatibility(
            &with_transient(Some(json!(["a"]))),
            &with_transient(Some(json!(["a", "b"]))),
            platform_version,
        )
        .expect_err("a transient diff must hard-error under v0");

        assert_matches!(
            error,
            ProtocolError::DataContractError(DataContractError::JsonSchema(
                JsonSchemaError::SchemaCompatibilityValidationError(message)
            )) if message == "schema keyword 'transient' at path '/transient/1' is not supported"
        );
    }

    fn with_property_constraints(rules: Option<serde_json::Value>) -> serde_json::Value {
        let mut schema = json!({
            "type": "object",
            "properties": {
                "a": {"type": "integer", "position": 0},
                "b": {"type": "integer", "position": 1},
            },
            "additionalProperties": false,
        });
        if let Some(rules) = rules {
            schema["propertyConstraints"] = rules;
        }
        schema
    }

    /// Every stored document was judged against the rules, so adding, removing or
    /// changing any part of one is incompatible, the operator and comparison keys
    /// inside a rule included: they are the declaration's data, not JSON Schema
    /// keywords.
    #[test]
    fn should_report_every_property_constraints_change_as_incompatible() {
        let platform_version = PlatformVersion::latest();
        let rule = json!({ "sum": { "lessThanOrEqual": [{ "add": ["a", "b"] }, 100] } });
        for (original, new, change_name, change_path) in [
            (None, Some(rule.clone()), "add", "/propertyConstraints"),
            (Some(rule.clone()), None, "remove", "/propertyConstraints"),
            (
                Some(rule.clone()),
                Some(json!({
                    "sum": { "lessThanOrEqual": [{ "add": ["a", "b"] }, 100] },
                    "order": { "lessThan": ["a", "b"] }
                })),
                "add",
                "/propertyConstraints/order",
            ),
            (
                Some(rule.clone()),
                Some(json!({ "sum": { "lessThanOrEqual": [{ "add": ["a", "b"] }, 99] } })),
                "replace",
                "/propertyConstraints/sum/lessThanOrEqual/1",
            ),
            (
                Some(rule.clone()),
                Some(json!({ "sum": { "lessThanOrEqual": [{ "add": ["a", "b", 1] }, 100] } })),
                "add",
                "/propertyConstraints/sum/lessThanOrEqual/0/add/2",
            ),
        ] {
            let result = validate_schema_compatibility(
                &with_property_constraints(original.clone()),
                &with_property_constraints(new.clone()),
                platform_version,
            )
            .expect("a propertyConstraints change is judged, not an unsupported keyword");
            assert_matches!(
                result.errors.as_slice(),
                [change] if change.name == change_name && change.path == change_path,
                "{original:?} -> {new:?}"
            );
        }

        let unchanged = with_property_constraints(Some(rule));
        assert!(
            validate_schema_compatibility(&unchanged, &unchanged, platform_version)
                .expect("an unchanged schema is judged")
                .is_valid()
        );
    }
}
