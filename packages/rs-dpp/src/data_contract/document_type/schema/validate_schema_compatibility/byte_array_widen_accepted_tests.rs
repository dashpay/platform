//! Security regression test documenting the validation gap that enables the
//! byteArray encoding-flip chain-halt (Dash Platform v4.0.0-rc.1).
//!
//! `DataContractUpdate` runs each document type's new schema through
//! [`validate_schema_compatibility`] (the same version-aware production entry
//! point exercised here). Widening a `byteArray` property's `maxItems`
//! (e.g. 32 -> 64), or removing `maxItems` entirely, is classified as a
//! BACKWARD-COMPATIBLE change and is therefore ACCEPTED.
//!
//! That acceptance is exactly what makes the encoding flip reachable on chain:
//! validation lets the update through, but the on-disk encoding of the property
//! flips (raw fixed-size -> varint-length-prefixed), which corrupts the decode
//! of already-stored documents (see `byte_array_encoding_flip_tests` in the
//! `property` module).
//!
//! This test PASSES today by asserting that the widening is reported as
//! compatible. It documents the gap; it is expected to keep passing even after
//! the decode-side fix, unless the fix is implemented by rejecting the update at
//! validation time (in which case these assertions should be updated).

use super::validate_schema_compatibility;
use platform_version::version::PlatformVersion;
use serde_json::json;

/// A fixed `byteArray` schema with `minItems == maxItems == 32`.
fn fixed_32_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "data": {
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "maxItems": 32,
                "position": 0
            }
        },
        "additionalProperties": false
    })
}

#[test]
fn update_validation_accepts_widening_max_items_32_to_64() {
    let platform_version = PlatformVersion::latest();

    let original = fixed_32_schema();
    let widened = json!({
        "type": "object",
        "properties": {
            "data": {
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "maxItems": 64,
                "position": 0
            }
        },
        "additionalProperties": false
    });

    let result = validate_schema_compatibility(&original, &widened, platform_version)
        .expect("compatibility validation must not error");

    assert!(
        result.is_valid(),
        "VULNERABILITY (validation gap): widening byteArray maxItems 32 -> 64 is \
         accepted as a compatible DataContractUpdate, even though it flips the \
         on-disk encoding and corrupts existing documents. Incompatibilities \
         reported: {:?}",
        result.errors
    );
    assert!(
        result.errors.is_empty(),
        "expected no incompatibilities for the maxItems widen, got: {:?}",
        result.errors
    );
}

#[test]
fn update_validation_accepts_removing_max_items() {
    let platform_version = PlatformVersion::latest();

    let original = fixed_32_schema();
    // maxItems removed entirely -> property becomes variable-length on disk.
    let removed = json!({
        "type": "object",
        "properties": {
            "data": {
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "position": 0
            }
        },
        "additionalProperties": false
    });

    let result = validate_schema_compatibility(&original, &removed, platform_version)
        .expect("compatibility validation must not error");

    assert!(
        result.is_valid(),
        "VULNERABILITY (validation gap): removing byteArray maxItems is accepted \
         as a compatible DataContractUpdate, even though it flips the on-disk \
         encoding. Incompatibilities reported: {:?}",
        result.errors
    );
}

/// Sanity control: SHRINKING maxItems (64 -> 32) is correctly rejected, proving
/// the validator is actually inspecting `maxItems` (so the acceptance of the
/// WIDENING above is a deliberate rule, not a path that ignores the keyword).
#[test]
fn update_validation_rejects_shrinking_max_items_control() {
    let platform_version = PlatformVersion::latest();

    let original = json!({
        "type": "object",
        "properties": {
            "data": {
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "maxItems": 64,
                "position": 0
            }
        },
        "additionalProperties": false
    });
    let shrunk = fixed_32_schema();

    let result = validate_schema_compatibility(&original, &shrunk, platform_version)
        .expect("compatibility validation must not error");

    assert!(
        !result.is_valid(),
        "shrinking maxItems 64 -> 32 should be rejected as incompatible; if this \
         is accepted the validator is not inspecting maxItems at all"
    );
}
