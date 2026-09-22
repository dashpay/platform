//! Updating a property bounded to a declared set of values (the schema's
//! `enum`). Existing documents must stay valid, so the set may grow and the
//! bound may be lifted, while removing or replacing a value, or bounding a
//! property that was unbounded, is an incompatible schema change.

use super::validate_schema_compatibility;
use platform_version::version::PlatformVersion;
use serde_json::json;

/// The `salad` document type with `dressing` bounded to `members`.
fn bounded_salad(members: serde_json::Value) -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "dressing": {
                "type": "string",
                "maxLength": 20,
                "enum": members,
                "position": 0
            }
        },
        "additionalProperties": false
    })
}

/// The `salad` document type with `dressing` unbounded.
fn unbounded_salad() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "dressing": {
                "type": "string",
                "maxLength": 20,
                "position": 0
            }
        },
        "additionalProperties": false
    })
}

fn three_dressings() -> serde_json::Value {
    json!(["butter", "margarine", "vinaigrette"])
}

fn assert_incompatible_on_the_bound(
    result: &crate::validation::SimpleValidationResult<super::IncompatibleJsonSchemaOperation>,
) {
    assert!(!result.is_valid(), "expected an incompatible change");
    assert!(
        result
            .errors
            .iter()
            .any(|operation| operation.path.starts_with("/properties/dressing/enum")),
        "the incompatibility must point at the bound, got {:?}",
        result.errors
    );
}

#[test]
fn update_may_add_an_allowed_value() {
    let result = validate_schema_compatibility(
        &bounded_salad(three_dressings()),
        &bounded_salad(json!(["butter", "margarine", "vinaigrette", "mayonnaise"])),
        PlatformVersion::latest(),
    )
    .expect("compatibility validation must not error");
    assert!(
        result.is_valid(),
        "adding a value keeps every existing document valid, got {:?}",
        result.errors
    );
}

#[test]
fn update_may_not_remove_an_allowed_value() {
    let result = validate_schema_compatibility(
        &bounded_salad(three_dressings()),
        &bounded_salad(json!(["butter", "margarine"])),
        PlatformVersion::latest(),
    )
    .expect("compatibility validation must not error");
    assert_incompatible_on_the_bound(&result);
}

#[test]
fn update_may_not_replace_an_allowed_value() {
    let result = validate_schema_compatibility(
        &bounded_salad(three_dressings()),
        &bounded_salad(json!(["butter", "margarine", "mayonnaise"])),
        PlatformVersion::latest(),
    )
    .expect("compatibility validation must not error");
    assert_incompatible_on_the_bound(&result);
}

#[test]
fn update_may_lift_the_bound() {
    let result = validate_schema_compatibility(
        &bounded_salad(three_dressings()),
        &unbounded_salad(),
        PlatformVersion::latest(),
    )
    .expect("compatibility validation must not error");
    assert!(
        result.is_valid(),
        "lifting the bound keeps every existing document valid, got {:?}",
        result.errors
    );
}

#[test]
fn update_may_not_bound_an_unbounded_property() {
    let result = validate_schema_compatibility(
        &unbounded_salad(),
        &bounded_salad(three_dressings()),
        PlatformVersion::latest(),
    )
    .expect("compatibility validation must not error");
    assert_incompatible_on_the_bound(&result);
}
