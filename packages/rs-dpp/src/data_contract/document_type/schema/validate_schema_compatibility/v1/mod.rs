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
//!
//! Every other keyword the document meta-schema admits and the shared rule set
//! has no rule for gets the same frozen rule ([`FROZEN_KEYWORDS_WITHOUT_A_SHARED_RULE`]).
//! A keyword that still has no rule is frozen as well: its change is reported
//! as incompatible instead of failing the update as an unsupported keyword.
//! Generation 0 fails on a diff under any of them as an unsupported keyword.

use crate::data_contract::document_type::property_names::{
    ACTION_FEES, CONTAINS, DOCUMENTS_AVERAGEABLE, DOCUMENTS_COUNTABLE, DOCUMENTS_SUMMABLE,
    ENTRY_PAYLOAD, INDEX_ONLY, KEEPS_PRICING_HISTORY, KEEPS_PURCHASE_HISTORY,
    KEEPS_TRANSFER_HISTORY, MAX_PROPERTIES, MIN_PROPERTIES, MODERATOR_ABILITIES,
    PROPERTY_CONSTRAINTS, RANGE_AVERAGEABLE, RANGE_COUNTABLE, RANGE_SUMMABLE, RETRACTED_WHEN,
    TOKEN_COST, TRANSIENT, TTL,
};
use crate::data_contract::document_type::schema::IncompatibleJsonSchemaOperation;
use crate::data_contract::errors::{DataContractError, JsonSchemaError};
use crate::data_contract::JsonValue;
use crate::validation::SimpleValidationResult;
use crate::ProtocolError;
use json_schema_compatibility_validator::error::{
    Error as CompatibilityError, UnsupportedSchemaKeywordError,
};
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
    // So does every keyword in `FROZEN_KEYWORDS_WITHOUT_A_SHARED_RULE`.
    let refers_to_rule = KEYWORD_COMPATIBILITY_RULES.get("refersTo");
    let frozen_doctype_rules = [
        "ownerRefersTo",
        "creatorRefersTo",
        TRANSIENT,
        PROPERTY_CONSTRAINTS,
    ]
    .into_iter()
    .chain(FROZEN_KEYWORDS_WITHOUT_A_SHARED_RULE)
    .filter_map(|keyword| refers_to_rule.map(|rule| (keyword, rule.clone())));

    Options {
        override_rules: CompatibilityRulesCollection::from_iter(
            [("required", required_rule)]
                .into_iter()
                .chain(frozen_doctype_rules),
        ),
    }
});

/// The keywords the document meta-schema admits that have no rule in the
/// shared rule set, which generation 0 also reads, and are not stripped by
/// [`prepared_for_diff`]. Without a rule, a diff under one fails as an
/// unsupported keyword: an internal error, where a contract update should get
/// a consensus error. Each is frozen, so adding, removing or changing it is an
/// incompatible change.
///
/// Where the document type parse reads the keyword, `validate_update` v1
/// compares the parsed values first and refuses a real change with
/// `DocumentTypeUpdateError`. What reaches this rule is then an edit the parse
/// reads the same, such as writing out a default or switching to the
/// `documentsAverageable` shorthand, refused like the same edit to
/// `documentsMutable` or `canBeDeleted`. `minProperties`, `maxProperties` and
/// `contains` have no parsed value; the first two are also admitted on object
/// properties and `contains` only on properties, where the rule applies too.
/// `$schema` needs no rule: the parse refuses a document type schema that
/// carries it and adds it only to the copy it validates.
///
/// A keyword missing from this list is still refused, by the fallback in
/// [`validate_schema_compatibility_v1`], but only the first change under it is
/// reported: the list keeps every change reported at its own path.
const FROZEN_KEYWORDS_WITHOUT_A_SHARED_RULE: [&str; 19] = [
    TOKEN_COST,
    TTL,
    ACTION_FEES,
    INDEX_ONLY,
    ENTRY_PAYLOAD,
    KEEPS_TRANSFER_HISTORY,
    KEEPS_PURCHASE_HISTORY,
    KEEPS_PRICING_HISTORY,
    DOCUMENTS_COUNTABLE,
    RANGE_COUNTABLE,
    DOCUMENTS_SUMMABLE,
    RANGE_SUMMABLE,
    DOCUMENTS_AVERAGEABLE,
    RANGE_AVERAGEABLE,
    MODERATOR_ABILITIES,
    RETRACTED_WHEN,
    MIN_PROPERTIES,
    MAX_PROPERTIES,
    CONTAINS,
];

/// The document type's own top-level keys whose changes are validated by
/// dedicated checks in `validate_update` v1 instead of the JSON diff:
/// `indices` (index definitions compared by name), `required`
/// (`validate_required_fields_update`, which admits new-property additions
/// annotated with `requiredSince`), and `immutable`
/// (`validate_immutable_fields_update`: the properties it lists without a
/// condition may only grow, and a condition may only be dropped for listing
/// the property without one). The differ has no rule for the last two at all
/// and would hard-error on any change to them.
const TOP_LEVEL_VALIDATED_KEYS: [&str; 3] = ["indices", "required", "immutable"];

/// The document type's own top-level lists of property names that the parse
/// reads as sets: `transient`, and `entryPayload`, whose properties are framed
/// in each stored entry in name order whatever order the list gives.
const TOP_LEVEL_NAME_SETS: [&str; 2] = [TRANSIENT, ENTRY_PAYLOAD];

/// Prepares a document type schema to be diffed: strips
/// [`TOP_LEVEL_VALIDATED_KEYS`], and sorts and deduplicates each list of
/// [`TOP_LEVEL_NAME_SETS`], so reordering or repeating names is no change.
/// Only the document type's own top-level keys are touched; a nested object
/// property's `required` array lives under `/properties/<name>/required` and
/// stays governed by the differ's frozen `required` rule, as do properties
/// named `indices`, `required`, `immutable`, `transient` or `entryPayload`.
fn prepared_for_diff(schema: &JsonValue) -> Cow<'_, JsonValue> {
    match schema {
        JsonValue::Object(map)
            if TOP_LEVEL_NAME_SETS
                .iter()
                .chain(TOP_LEVEL_VALIDATED_KEYS.iter())
                .any(|key| map.contains_key(*key)) =>
        {
            let mut map = map.clone();
            for key in TOP_LEVEL_VALIDATED_KEYS {
                map.remove(key);
            }
            for key in TOP_LEVEL_NAME_SETS {
                if let Some(JsonValue::Array(names)) = map.get_mut(key) {
                    names.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
                    names.dedup();
                }
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
/// loosening of what `immutable` freezes before this check runs. A future version
/// table that bumps one without the other would let those changes bypass
/// compatibility validation entirely.
pub(super) fn validate_schema_compatibility_v1(
    original_schema: &JsonValue,
    new_schema: &JsonValue,
) -> Result<SimpleValidationResult<IncompatibleJsonSchemaOperation>, ProtocolError> {
    let original_schema = prepared_for_diff(original_schema);
    let new_schema = prepared_for_diff(new_schema);

    match validate_schemas_compatibility(&original_schema, &new_schema, OPTIONS.deref()) {
        Ok(result) => {
            let errors = result
                .into_changes()
                .into_iter()
                .map(|change| IncompatibleJsonSchemaOperation {
                    name: change.name().to_string(),
                    path: change.path().to_string(),
                })
                .collect::<Vec<_>>();

            Ok(SimpleValidationResult::new_with_errors(errors))
        }
        // A keyword with no rule at all is frozen like those listed: its change
        // is an incompatible one, not an internal error. The validator stops at
        // it, so it is the only change reported. The operation is read off the
        // two schemas as the diff chose it: a path the original lacks was added,
        // one the new schema lacks was removed, and any other was replaced.
        Err(CompatibilityError::UnsupportedSchemaKeyword(UnsupportedSchemaKeywordError {
            path,
            ..
        })) => {
            let name = match (original_schema.pointer(&path), new_schema.pointer(&path)) {
                (None, _) => "add",
                (_, None) => "remove",
                _ => "replace",
            };
            Ok(SimpleValidationResult::new_with_error(
                IncompatibleJsonSchemaOperation {
                    name: name.to_string(),
                    path,
                },
            ))
        }
        Err(error) => Err(ProtocolError::DataContractError(
            DataContractError::JsonSchema(JsonSchemaError::SchemaCompatibilityValidationError(
                error.to_string(),
            )),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::super::validate_schema_compatibility;
    use super::{OPTIONS, TOP_LEVEL_VALIDATED_KEYS};
    use crate::data_contract::errors::{DataContractError, JsonSchemaError};
    use crate::ProtocolError;
    use assert_matches::assert_matches;
    use json_schema_compatibility_validator::KEYWORD_COMPATIBILITY_RULES;
    use platform_version::version::{PlatformVersion, PLATFORM_VERSIONS};
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
            "immutable": ["a", { "property": "b", "when": { "present": "$old.b" } }],
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

    fn document_type_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "a": {"type": "integer", "position": 0},
                "list": {"type": "array", "items": {"type": "integer"}, "position": 1},
                "object": {
                    "type": "object",
                    "properties": {"b": {"type": "integer", "position": 0}},
                    "additionalProperties": false,
                    "position": 2
                },
            },
            "additionalProperties": false,
        })
    }

    /// Where a keyword sits, a value, another value, and the path of the first
    /// incompatible change reported between the two.
    type FrozenKeywordCase = (
        &'static str,
        serde_json::Value,
        serde_json::Value,
        &'static str,
    );

    /// Every keyword the differ freezes with no shared rule, at the top of the
    /// document type or in a property's schema.
    fn frozen_keyword_cases() -> Vec<FrozenKeywordCase> {
        let mut cases: Vec<FrozenKeywordCase> = vec![
            (
                "/tokenCost",
                json!({"create": {"tokenPosition": 0, "amount": 1}}),
                json!({"create": {"tokenPosition": 0, "amount": 1, "effect": 0}}),
                "/tokenCost/create/effect",
            ),
            (
                "/actionFees",
                json!({"create": {"owner": 10}}),
                json!({"create": {"owner": 10, "moderators": 0}}),
                "/actionFees/create/moderators",
            ),
            (
                "/entryPayload",
                json!(["a", "b"]),
                json!(["a", "c"]),
                "/entryPayload/1",
            ),
            (
                "/moderatorAbilities",
                json!({"delete": true}),
                json!({"delete": false}),
                "/moderatorAbilities/delete",
            ),
            (
                "/moderatorAbilities",
                json!({"delete": true, "deleteWithin": 3600}),
                json!({"delete": true, "deleteWithin": 7200}),
                "/moderatorAbilities/deleteWithin",
            ),
            (
                "/moderatorAbilities",
                json!({"changeFields": ["a", "b"]}),
                json!({"changeFields": ["a", "c"]}),
                "/moderatorAbilities/changeFields/1",
            ),
            (
                "/properties/list/contains",
                json!({"minimum": 1}),
                json!({"minimum": 0}),
                "/properties/list/contains/minimum",
            ),
        ];
        // Scalars, whose change is reported at the keyword itself
        let scalars = [
            ("/ttl", json!(86400), json!(3600)),
            ("/indexOnly", json!(true), json!(false)),
            ("/keepsTransferHistory", json!(true), json!(false)),
            ("/keepsPurchaseHistory", json!(true), json!(false)),
            ("/keepsPricingHistory", json!(true), json!(false)),
            ("/documentsCountable", json!(true), json!(false)),
            ("/rangeCountable", json!(true), json!(false)),
            ("/documentsSummable", json!("a"), json!("b")),
            ("/rangeSummable", json!(true), json!(false)),
            ("/documentsAverageable", json!("a"), json!("b")),
            ("/rangeAverageable", json!(true), json!(false)),
            ("/minProperties", json!(1), json!(0)),
            ("/maxProperties", json!(2), json!(3)),
            ("/properties/object/minProperties", json!(1), json!(0)),
            ("/properties/object/maxProperties", json!(1), json!(2)),
        ];
        cases.extend(
            scalars
                .into_iter()
                .map(|(pointer, value, other_value)| (pointer, value, other_value, pointer)),
        );
        cases
    }

    fn with_pointer(pointer: &str, value: Option<serde_json::Value>) -> serde_json::Value {
        let mut schema = document_type_schema();
        let (parent, key) = pointer.rsplit_once('/').expect("a pointer has a parent");
        let parent = schema
            .pointer_mut(parent)
            .and_then(serde_json::Value::as_object_mut)
            .expect("the parent is an object");
        if let Some(value) = value {
            parent.insert(key.to_string(), value);
        }
        schema
    }

    /// Meta-schema v3 admits these keywords, which the shared rule set has no
    /// rule for: each is frozen, so adding, removing or changing one, even to
    /// a value the parse reads the same, is an incompatible change and not an
    /// unsupported keyword. Where the parse reads a value, `validate_update`
    /// refuses a real change before this check runs.
    #[test]
    fn should_report_every_change_to_a_keyword_without_a_shared_rule_as_incompatible() {
        let platform_version = PlatformVersion::latest();
        for (pointer, value, other_value, changed_path) in frozen_keyword_cases() {
            // Adding, removing, then changing the value: the first incompatible
            // change reported is at the keyword, then inside its value
            for (original, new, first_change_path) in [
                (None, Some(value.clone()), pointer),
                (Some(value.clone()), None, pointer),
                (Some(value.clone()), Some(other_value), changed_path),
            ] {
                let result = validate_schema_compatibility(
                    &with_pointer(pointer, original.clone()),
                    &with_pointer(pointer, new.clone()),
                    platform_version,
                )
                .unwrap_or_else(|error| {
                    panic!("{pointer}: {original:?} -> {new:?} must be judged, got {error:?}")
                });
                assert_matches!(
                    result.errors.as_slice(),
                    [change, ..] if change.path == first_change_path,
                    "{pointer}: {original:?} -> {new:?}"
                );
            }

            let unchanged = with_pointer(pointer, Some(value));
            assert!(
                validate_schema_compatibility(&unchanged, &unchanged, platform_version)
                    .expect("an unchanged schema is judged")
                    .is_valid(),
                "{pointer}"
            );
        }
    }

    /// Every keyword the document meta-schema admits, at the top of a document
    /// type, in a property's schema or in a typed array's element schema, is
    /// judged by a rule of its own or stripped before the diff, for every
    /// protocol version that selects this generation. A keyword added to the
    /// meta-schema without a rule fails here, and so does a new meta-schema
    /// diffed by this generation until it is listed below.
    #[test]
    fn should_have_a_rule_for_every_keyword_the_meta_schema_admits() {
        let keywords = |schema: &serde_json::Value| -> Vec<String> {
            schema["properties"]
                .as_object()
                .expect("the schema declares its keywords")
                .keys()
                .cloned()
                .collect()
        };
        let has_rule = |keyword: &str| {
            OPTIONS.override_rules.contains_key(keyword)
                || KEYWORD_COMPATIBILITY_RULES.contains_key(keyword)
        };

        for platform_version in PLATFORM_VERSIONS {
            let schema_versions = &platform_version
                .dpp
                .contract_versions
                .document_type_versions
                .schema;
            if schema_versions.validate_schema_compatibility != 1 {
                continue;
            }
            let meta_schema: serde_json::Value = match schema_versions.document_type_schema {
                3 => serde_json::from_str(include_str!(
                    "../../../../../../schema/meta_schemas/document/v3/document-meta.json"
                ))
                .expect("the v3 document meta-schema is JSON"),
                version => panic!(
                    "protocol version {} diffs document meta-schema {version} with this \
                     generation: list it here",
                    platform_version.protocol_version
                ),
            };

            for keyword in keywords(&meta_schema) {
                // The parse refuses a schema carrying `$schema`, so no diff reaches it
                if keyword == "$schema" || TOP_LEVEL_VALIDATED_KEYS.contains(&keyword.as_str()) {
                    continue;
                }
                assert!(
                    has_rule(&keyword),
                    "top-level keyword {keyword} has no rule"
                );
            }
            for definition in ["documentSchema", "documentArrayItem"] {
                for keyword in keywords(&meta_schema["$defs"][definition]) {
                    assert!(
                        has_rule(&keyword),
                        "{definition} keyword {keyword} has no rule"
                    );
                }
            }
        }
    }

    /// A keyword with no rule at all is refused as an incompatible change, with
    /// the operation the diff made, instead of failing as an unsupported
    /// keyword. The meta-schema admits no such keyword today; the fallback is
    /// what keeps a later one from failing the update with an internal error.
    #[test]
    fn should_report_a_change_under_a_keyword_with_no_rule_as_incompatible() {
        let platform_version = PlatformVersion::latest();
        for (pointer, original, new, change_name, change_path) in [
            ("/unruled", None, Some(json!(1)), "add", "/unruled"),
            ("/unruled", Some(json!(1)), None, "remove", "/unruled"),
            (
                "/unruled",
                Some(json!(1)),
                Some(json!(2)),
                "replace",
                "/unruled",
            ),
            (
                "/unruled",
                Some(json!({"a": 1})),
                Some(json!({"a": 1, "b": 2})),
                "add",
                "/unruled/b",
            ),
            (
                "/properties/a/unruled",
                Some(json!([1, 2])),
                Some(json!([1])),
                "remove",
                "/properties/a/unruled/1",
            ),
        ] {
            let result = validate_schema_compatibility(
                &with_pointer(pointer, original.clone()),
                &with_pointer(pointer, new.clone()),
                platform_version,
            )
            .unwrap_or_else(|error| {
                panic!("{pointer}: {original:?} -> {new:?} must be judged, got {error:?}")
            });
            assert_matches!(
                result.errors.as_slice(),
                [change] if change.name == change_name && change.path == change_path,
                "{pointer}: {original:?} -> {new:?}"
            );
        }
    }

    /// The parse reads `entryPayload` as a set, as it does `transient`, so
    /// reordering or repeating names changes nothing a stored entry depends on.
    #[test]
    fn should_accept_a_reordered_or_repeated_entry_payload() {
        let platform_version = PlatformVersion::latest();
        for new in [json!(["b", "a"]), json!(["a", "b", "a"])] {
            let result = validate_schema_compatibility(
                &with_pointer("/entryPayload", Some(json!(["a", "b"]))),
                &with_pointer("/entryPayload", Some(new.clone())),
                platform_version,
            )
            .expect("an entryPayload change is judged, not an unsupported keyword");
            assert!(result.is_valid(), "{new:?}: {:?}", result.errors);
        }
    }

    // Replay-safety pin: protocol version 13 dispatches to v0, where a diff
    // under a keyword without a shared rule still hits the unsupported-keyword
    // hard error.
    #[test]
    fn should_hard_error_on_a_token_cost_diff_at_protocol_version_13() {
        let platform_version = PlatformVersion::get(13).expect("protocol version 13 must exist");
        let error = validate_schema_compatibility(
            &with_pointer(
                "/tokenCost",
                Some(json!({"create": {"tokenPosition": 0, "amount": 1}})),
            ),
            &with_pointer(
                "/tokenCost",
                Some(json!({"create": {"tokenPosition": 0, "amount": 2}})),
            ),
            platform_version,
        )
        .expect_err("a tokenCost diff must hard-error under v0");

        assert_matches!(
            error,
            ProtocolError::DataContractError(DataContractError::JsonSchema(
                JsonSchemaError::SchemaCompatibilityValidationError(message)
            )) if message == "schema keyword 'tokenCost' at path '/tokenCost/create/amount' is not supported"
        );
    }
}
