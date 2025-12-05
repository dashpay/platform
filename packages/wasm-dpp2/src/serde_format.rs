//! Format-aware serialization context for WASM.
//!
//! This module provides a thread-local context mechanism that allows types like
//! `IdentifierWasm` to serialize differently depending on whether the output
//! target is JSON (string representation) or WASM objects (bytes/Uint8Array).
//!
//! # Safety
//!
//! This approach is safe in WASM because WebAssembly in browsers/Node.js is
//! single-threaded by default. JavaScript's event loop ensures that only one
//! JS→WASM call executes at a time, so there's no risk of concurrent format
//! context corruption.

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value as JsonValue;
use std::cell::Cell;
use wasm_bindgen::JsValue;

/// Serialization format context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SerdeFormat {
    /// JSON serialization: identifiers become Base58 strings, etc.
    Json,
    /// WASM/object serialization: identifiers become Uint8Array bytes, etc.
    #[default]
    Wasm,
}

thread_local! {
    static SERDE_FORMAT: Cell<SerdeFormat> = const { Cell::new(SerdeFormat::Wasm) };
}

/// Returns the current serialization format context.
///
/// Defaults to `SerdeFormat::Wasm` if no context has been set.
pub fn current_format() -> SerdeFormat {
    SERDE_FORMAT.with(|f| f.get())
}

/// RAII guard that sets the serialization format for the duration of its lifetime.
///
/// When dropped, restores the previous format. This ensures cleanup even if
/// serialization panics.
struct FormatGuard {
    previous: SerdeFormat,
}

impl FormatGuard {
    fn new(format: SerdeFormat) -> Self {
        let previous = SERDE_FORMAT.with(|f| {
            let prev = f.get();
            f.set(format);
            prev
        });
        FormatGuard { previous }
    }
}

impl Drop for FormatGuard {
    fn drop(&mut self) {
        SERDE_FORMAT.with(|f| f.set(self.previous));
    }
}

/// Serialize a value to `serde_json::Value` with JSON format context.
///
/// Types that check `current_format()` will serialize in JSON-friendly format
/// (e.g., identifiers as Base58 strings).
pub fn to_json_value<T: Serialize>(value: &T) -> Result<JsonValue, serde_json::Error> {
    let _guard = FormatGuard::new(SerdeFormat::Json);
    serde_json::to_value(value)
}

/// Deserialize from `serde_json::Value` with JSON format context.
pub fn from_json_value<T: DeserializeOwned>(value: JsonValue) -> Result<T, serde_json::Error> {
    let _guard = FormatGuard::new(SerdeFormat::Json);
    serde_json::from_value(value)
}

/// Serialize a value to JSON string with JSON format context.
pub fn to_json_string<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    let _guard = FormatGuard::new(SerdeFormat::Json);
    serde_json::to_string(value)
}

/// Serialize a value to pretty-printed JSON string with JSON format context.
pub fn to_json_string_pretty<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    let _guard = FormatGuard::new(SerdeFormat::Json);
    serde_json::to_string_pretty(value)
}

/// Deserialize from JSON string with JSON format context.
pub fn from_json_str<T: DeserializeOwned>(s: &str) -> Result<T, serde_json::Error> {
    let _guard = FormatGuard::new(SerdeFormat::Json);
    serde_json::from_str(s)
}

/// Serialize a value to `JsValue` with WASM format context.
///
/// Types that check `current_format()` will serialize in WASM-friendly format
/// (e.g., identifiers as bytes → Uint8Array).
pub fn to_wasm_value<T: Serialize>(value: &T) -> Result<JsValue, serde_wasm_bindgen::Error> {
    let _guard = FormatGuard::new(SerdeFormat::Wasm);
    serde_wasm_bindgen::to_value(value)
}

/// Deserialize from `JsValue` with WASM format context.
pub fn from_wasm_value<T: DeserializeOwned>(js: JsValue) -> Result<T, serde_wasm_bindgen::Error> {
    let _guard = FormatGuard::new(SerdeFormat::Wasm);
    serde_wasm_bindgen::from_value(js)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_guard_restores_previous() {
        // Default is Wasm
        assert_eq!(current_format(), SerdeFormat::Wasm);

        {
            let _guard = FormatGuard::new(SerdeFormat::Json);
            assert_eq!(current_format(), SerdeFormat::Json);

            {
                let _inner_guard = FormatGuard::new(SerdeFormat::Wasm);
                assert_eq!(current_format(), SerdeFormat::Wasm);
            }

            // Inner guard dropped, should restore to Json
            assert_eq!(current_format(), SerdeFormat::Json);
        }

        // Outer guard dropped, should restore to Wasm
        assert_eq!(current_format(), SerdeFormat::Wasm);
    }
}
