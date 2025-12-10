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

use crate::error::{WasmDppError, WasmDppResult};
use js_sys::Object;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value as JsonValue;
use std::cell::Cell;
use wasm_bindgen::prelude::*;
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

/// Serialize a value to `JsValue` using JSON-compatible serializer.
///
/// This ensures objects become plain JS objects (not Maps) which is important
/// for JSON serialization compatibility.
pub fn to_js_value_json_compatible<T: Serialize>(
    value: &T,
) -> Result<JsValue, serde_wasm_bindgen::Error> {
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    value.serialize(&serializer)
}

/// Recursively converts BigInt values to strings in a JsValue.
///
/// This is necessary because serde_wasm_bindgen cannot convert BigInt values larger than
/// Number.MAX_SAFE_INTEGER to JSON numbers.
pub fn convert_bigints_to_strings(value: &JsValue) -> WasmDppResult<JsValue> {
    if value.is_bigint() {
        // Convert BigInt to string using js_sys::BigInt
        let bigint: js_sys::BigInt = value.clone().unchecked_into();
        let bigint_str = bigint
            .to_string(10)
            .map(|s| s.into())
            .unwrap_or_else(|_| "0".to_string());
        Ok(JsValue::from_str(&bigint_str))
    } else if js_sys::Array::is_array(value) {
        // Handle arrays recursively
        let arr = js_sys::Array::from(value);
        let new_arr = js_sys::Array::new();
        for i in 0..arr.length() {
            let elem = arr.get(i);
            let converted = convert_bigints_to_strings(&elem)?;
            new_arr.push(&converted);
        }
        Ok(new_arr.into())
    } else if value.is_object() && !value.is_null() {
        // Handle objects recursively
        let obj = Object::from(value.clone());
        let new_obj = Object::new();
        let keys = Object::keys(&obj);
        for i in 0..keys.length() {
            let key = keys.get(i);
            if key.as_string().is_some() {
                let prop_value = js_sys::Reflect::get(value, &key).map_err(|e| {
                    WasmDppError::serialization(format!("Failed to get property: {:?}", e))
                })?;
                let converted = convert_bigints_to_strings(&prop_value)?;
                js_sys::Reflect::set(&new_obj, &key, &converted).map_err(|e| {
                    WasmDppError::serialization(format!("Failed to set property: {:?}", e))
                })?;
            }
        }
        Ok(new_obj.into())
    } else {
        // Return primitive values as-is
        Ok(value.clone())
    }
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
