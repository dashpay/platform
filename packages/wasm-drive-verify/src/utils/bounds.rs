//! Input bounds checking utilities

use crate::utils::error::{format_error_with_context, ErrorCategory};
use wasm_bindgen::JsValue;

/// Maximum allowed array length for input validation
pub const MAX_ARRAY_LENGTH: usize = 10_000;

/// Maximum allowed nested depth for object parsing
#[allow(dead_code)]
pub const MAX_NESTED_DEPTH: usize = 100;

/// Maximum allowed keys in an object
pub const MAX_OBJECT_KEYS: usize = 1_000;

/// Check if an array length is within bounds
pub fn check_array_bounds(length: usize, name: &str) -> Result<(), JsValue> {
    if length > MAX_ARRAY_LENGTH {
        return Err(format_error_with_context(
            ErrorCategory::BoundsError,
            name,
            &format!(
                "array length {} exceeds maximum of {}",
                length, MAX_ARRAY_LENGTH
            ),
        ));
    }
    Ok(())
}

/// Check if object key count is within bounds
pub fn check_object_bounds(key_count: usize, name: &str) -> Result<(), JsValue> {
    if key_count > MAX_OBJECT_KEYS {
        return Err(format_error_with_context(
            ErrorCategory::BoundsError,
            name,
            &format!(
                "object key count {} exceeds maximum of {}",
                key_count, MAX_OBJECT_KEYS
            ),
        ));
    }
    Ok(())
}
