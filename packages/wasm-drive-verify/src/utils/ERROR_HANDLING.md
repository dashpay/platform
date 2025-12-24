# Error Handling Standards

This document describes the standardized error handling pattern used throughout the wasm-drive-verify package.

## Error Categories

All errors are categorized into one of the following types:

- `InvalidInput` - Invalid input parameter
- `DeserializationError` - Deserialization failure
- `VerificationError` - Verification failure
- `PlatformVersionError` - Platform version error
- `ConversionError` - Type conversion error
- `NotFoundError` - Not found error
- `BoundsError` - Bounds exceeded error

## Error Formatting Functions

### `format_error(category: ErrorCategory, details: &str) -> JsValue`
Use for simple error messages with a category and details.

Example:
```rust
format_error(ErrorCategory::InvalidInput, "identity_id must be 32 bytes")
```

### `format_error_with_context(category: ErrorCategory, context: &str, details: &str) -> JsValue`
Use when you need to provide additional context about where the error occurred.

Example:
```rust
format_error_with_context(
    ErrorCategory::BoundsError,
    "where_clauses",
    &format!("array length {} exceeds maximum of {}", length, MAX_ARRAY_LENGTH)
)
```

### `format_result_error<E: Debug>(category: ErrorCategory, error: E) -> JsValue`
Use when converting from a Result's error type.

Example:
```rust
PlatformVersion::get(platform_version_number)
    .map_err(|e| format_result_error(ErrorCategory::PlatformVersionError, e))?
```

### `format_result_error_with_context<E: Debug>(category: ErrorCategory, context: &str, error: E) -> JsValue`
Use when converting from a Result's error with additional context.

Example:
```rust
contract.document_type_for_name(document_type_name)
    .map_err(|e| format_result_error_with_context(
        ErrorCategory::NotFoundError, 
        document_type_name, 
        e
    ))?
```

## Migration Guide

To update existing error handling:

1. Add import: `use crate::utils::error::{format_error, format_result_error, ErrorCategory};`
2. Replace `JsValue::from_str("error message")` with `format_error(ErrorCategory::X, "error message")`
3. Replace `JsValue::from_str(&format!("error: {:?}", e))` with `format_result_error(ErrorCategory::X, e)`

## Examples

### Before:
```rust
.map_err(|_| JsValue::from_str("Invalid identity_id length. Expected 32 bytes."))?
```

### After:
```rust
.map_err(|_| format_error(ErrorCategory::InvalidInput, "identity_id must be 32 bytes"))?
```

### Before:
```rust
.map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?
```

### After:
```rust
.map_err(|e| format_result_error(ErrorCategory::VerificationError, e))?
```