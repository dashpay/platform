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

    Options {
        override_rules: CompatibilityRulesCollection::from_iter([("required", required_rule)]),
    }
});

/// Strips the two top-level keys whose changes are validated by dedicated
/// checks in `validate_update` v1 instead of the JSON diff: `indices`
/// (index definitions compared by name) and `required`
/// (`validate_required_fields_update`, which admits new-property additions
/// annotated with `requiredSince`). Only the document type's own top-level
/// keys are removed; a nested object property's `required` array lives under
/// `/properties/<name>/required` and stays governed by the differ's frozen
/// `required` rule, as do properties named `indices` or `required`.
fn without_top_level_validated_keys(schema: &JsonValue) -> Cow<'_, JsonValue> {
    match schema {
        JsonValue::Object(map) if map.contains_key("indices") || map.contains_key("required") => {
            let mut map = map.clone();
            map.remove("indices");
            map.remove("required");
            Cow::Owned(JsonValue::Object(map))
        }
        _ => Cow::Borrowed(schema),
    }
}

/// Pairing invariant: stripping `indices` and top-level `required`
/// unconditionally is only safe because every `PlatformVersion` that selects
/// this generation (`validate_schema_compatibility: 1`) also selects a
/// `validate_update` generation of at least 1
/// (`dpp.validation.document_type.validate_update`), which rejects every
/// real index change and every disallowed required-set change before this
/// check runs. A future version table that bumps one without the other
/// would let index or required changes bypass compatibility validation
/// entirely.
pub(super) fn validate_schema_compatibility_v1(
    original_schema: &JsonValue,
    new_schema: &JsonValue,
) -> Result<SimpleValidationResult<IncompatibleJsonSchemaOperation>, ProtocolError> {
    let original_schema = without_top_level_validated_keys(original_schema);
    let new_schema = without_top_level_validated_keys(new_schema);

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
}
