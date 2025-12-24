//! Standardized error formatting utilities

use wasm_bindgen::JsValue;

/// Error categories for consistent error reporting
pub enum ErrorCategory {
    /// Invalid input parameter
    InvalidInput,
    /// Deserialization failure
    DeserializationError,
    /// Verification failure
    VerificationError,
    /// Platform version error
    PlatformVersionError,
    /// Type conversion error
    ConversionError,
    /// Not found error
    NotFoundError,
    /// Bounds exceeded error
    BoundsError,
}

impl ErrorCategory {
    fn prefix(&self) -> &'static str {
        match self {
            ErrorCategory::InvalidInput => "Invalid input",
            ErrorCategory::DeserializationError => "Deserialization failed",
            ErrorCategory::VerificationError => "Verification failed",
            ErrorCategory::PlatformVersionError => "Platform version error",
            ErrorCategory::ConversionError => "Type conversion failed",
            ErrorCategory::NotFoundError => "Not found",
            ErrorCategory::BoundsError => "Bounds exceeded",
        }
    }
}

/// Create a standardized error message
pub fn format_error(category: ErrorCategory, details: &str) -> JsValue {
    JsValue::from_str(&format!("{}: {}", category.prefix(), details))
}

/// Create a standardized error message with context
pub fn format_error_with_context(category: ErrorCategory, context: &str, details: &str) -> JsValue {
    JsValue::from_str(&format!("{} ({}): {}", category.prefix(), context, details))
}

/// Create a standardized error from a Result's error
pub fn format_result_error<E: std::fmt::Debug>(category: ErrorCategory, error: E) -> JsValue {
    JsValue::from_str(&format!("{}: {:?}", category.prefix(), error))
}

/// Create a standardized error from a Result's error with context
pub fn format_result_error_with_context<E: std::fmt::Debug>(
    category: ErrorCategory,
    context: &str,
    error: E,
) -> JsValue {
    JsValue::from_str(&format!("{} ({}): {:?}", category.prefix(), context, error))
}
