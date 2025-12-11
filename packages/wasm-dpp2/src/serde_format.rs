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
use dpp::platform_value;
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

/// Convert JsValue to serde_json::Value, handling BigInt values.
///
/// This function converts BigInt values to strings before conversion, which is the standard
/// approach for serializing large integers in JSON.
pub fn js_value_to_json(value: &JsValue) -> WasmDppResult<JsonValue> {
    let converted = convert_bigints_to_strings(value)?;
    serde_wasm_bindgen::from_value(converted)
        .map_err(|e| WasmDppError::serialization(format!("Failed to convert JsValue to JSON: {}", e)))
}

/// Convert serde_json::Value to JsValue using JSON-compatible serialization.
///
/// This ensures objects become plain JS objects (not Maps).
pub fn json_to_js_value(value: &JsonValue) -> WasmDppResult<JsValue> {
    to_js_value_json_compatible(value)
        .map_err(|e| WasmDppError::serialization(format!("Failed to convert JSON to JsValue: {}", e)))
}

/// Deserialize from JsValue with JSON format context, handling BigInt values.
///
/// This is useful for `fromJSON()` methods that receive JsValue from JavaScript.
pub fn from_js_value_json<T: DeserializeOwned>(value: JsValue) -> WasmDppResult<T> {
    let converted = convert_bigints_to_strings(&value)?;
    let json: JsonValue = serde_wasm_bindgen::from_value(converted)
        .map_err(|e| WasmDppError::serialization(format!("Failed to convert JsValue: {}", e)))?;
    from_json_value(json)
        .map_err(|e| WasmDppError::serialization(format!("Failed to deserialize JSON: {}", e)))
}

/// Serialize to JsValue with JSON format context.
///
/// This combines JSON format context (for proper identifier serialization) with
/// JSON-compatible output (plain objects instead of Maps).
pub fn to_js_value_json<T: Serialize>(value: &T) -> WasmDppResult<JsValue> {
    let json = to_json_value(value)
        .map_err(|e| WasmDppError::serialization(format!("Failed to serialize to JSON: {}", e)))?;
    json_to_js_value(&json)
}

/// Recursively normalizes a JsValue for JSON conversion.
///
/// This converts:
/// - BigInt values to strings (JSON doesn't support BigInt natively)
/// - Uint8Array to plain arrays (so they serialize as JSON number arrays)
///
/// Performance: Uses fast path for primitives, only recursively processes objects/arrays.
pub fn convert_bigints_to_strings(value: &JsValue) -> WasmDppResult<JsValue> {
    // Fast path: primitives that can't contain BigInt
    if value.is_string()
        || value.as_f64().is_some()
        || value.is_null()
        || value.is_undefined()
        || value.as_bool().is_some()
    {
        return Ok(value.clone());
    }

    if value.is_bigint() {
        let bigint: js_sys::BigInt = value.clone().unchecked_into();
        let bigint_str = bigint
            .to_string(10)
            .map(|s| s.into())
            .unwrap_or_else(|_| "0".to_string());
        return Ok(JsValue::from_str(&bigint_str));
    }

    // Convert Uint8Array to plain array for JSON compatibility
    if value.is_instance_of::<js_sys::Uint8Array>() {
        let uint8_array: js_sys::Uint8Array = value.clone().unchecked_into();
        let plain_array = js_sys::Array::from(&uint8_array);
        return Ok(plain_array.into());
    }

    if js_sys::Array::is_array(value) {
        let arr = js_sys::Array::from(value);
        let new_arr = js_sys::Array::new();
        for i in 0..arr.length() {
            let elem = arr.get(i);
            let converted = convert_bigints_to_strings(&elem)?;
            new_arr.push(&converted);
        }
        return Ok(new_arr.into());
    }

    if value.is_object() && !value.is_null() {
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
        return Ok(new_obj.into());
    }

    // Anything else (Symbol, Function, etc.): pass through
    Ok(value.clone())
}

/// Serialize to JsValue as a JS object (non-human-readable).
///
/// Uses the serde-wasm-bindgen serializer with `is_human_readable() -> false`,
/// so types like OutPoint serialize as bytes (Uint8Array).
/// Uses `serialize_large_number_types_as_bigints(true)` for u64/i64 -> BigInt.
pub fn to_object<T: Serialize>(value: &T) -> WasmDppResult<JsValue> {
    let serializer = serde_wasm_bindgen::Serializer::new()
        .serialize_maps_as_objects(true)
        .serialize_bytes_as_arrays(false)
        .serialize_large_number_types_as_bigints(true);
    value
        .serialize(&serializer)
        .map_err(|e| WasmDppError::serialization(format!("toObject: {}", e)))
}

/// Deserialize from JsValue (non-human-readable).
///
/// Uses the serde-wasm-bindgen deserializer with `is_human_readable() -> false`,
/// so types like OutPoint expect bytes (Uint8Array).
pub fn from_object<T: DeserializeOwned>(value: JsValue) -> WasmDppResult<T> {
    serde_wasm_bindgen::from_value(value)
        .map_err(|e| WasmDppError::serialization(format!("fromObject: {}", e)))
}

/// Serialize to JsValue as JSON-compatible (human-readable).
///
/// Uses `serialize_human_readable(true)` so types like Identifier serialize as base58 strings,
/// BinaryData as base64 strings, etc.
pub fn to_json<T: Serialize>(value: &T) -> WasmDppResult<JsValue> {
    let serializer = serde_wasm_bindgen::Serializer::json_compatible()
        .serialize_human_readable(true);
    value
        .serialize(&serializer)
        .map_err(|e| WasmDppError::serialization(format!("toJSON: {}", e)))
}

/// Deserialize from JsValue (human-readable JSON).
///
/// Uses the human-readable deserializer with `is_human_readable() -> true`,
/// so types like BinaryData expect base64 strings.
pub fn from_json<T: DeserializeOwned>(value: JsValue) -> WasmDppResult<T> {
    serde_wasm_bindgen::from_value_json(value)
        .map_err(|e| WasmDppError::serialization(format!("fromJSON: {}", e)))
}

/// Serialize platform_value::Value to JsValue as a JS object (non-human-readable).
///
/// Uses serialize_maps_as_objects(true) to ensure objects are plain JS objects.
/// Uses `serialize_bytes_as_arrays(false)` so bytes become Uint8Array (expected by JS API).
/// Uses `serialize_large_number_types_as_bigints(true)` for u64/i64 -> BigInt.
pub fn platform_value_to_object(value: &platform_value::Value) -> WasmDppResult<JsValue> {
    let serializer = serde_wasm_bindgen::Serializer::new()
        .serialize_maps_as_objects(true)
        .serialize_bytes_as_arrays(false)
        .serialize_large_number_types_as_bigints(true);
    value
        .serialize(&serializer)
        .map_err(|e| WasmDppError::serialization(format!("platform_value_to_object: {}", e)))
}

/// Deserialize JsValue to platform_value::Value.
///
/// serde-wasm-bindgen's deserialize_any handles Uint8Array via visit_byte_buf, which creates
/// Value::Bytes. BigInt is handled via visit_i64/visit_u64.
pub fn platform_value_from_object(value: JsValue) -> WasmDppResult<platform_value::Value> {
    serde_wasm_bindgen::from_value(value)
        .map_err(|e| WasmDppError::serialization(format!("platform_value_from_object: {}", e)))
}

/// Recursively normalizes a JsValue for deserialization to platform_value::Value.
///
/// This converts:
/// - BigInt values to strings
/// - Uint8Array to plain JS arrays (so they deserialize as Value::Array which can be
///   interpreted as bytes by schema-aware code)
///
/// This is necessary because serde_wasm_bindgen's deserialize_any treats Uint8Array as
/// a sequence, but platform_value::Value's visitor doesn't handle visit_bytes from
/// deserialize_any properly.
pub fn normalize_js_value_for_platform_value(value: &JsValue) -> WasmDppResult<JsValue> {
    // Fast path: primitives
    if value.is_string()
        || value.as_f64().is_some()
        || value.is_null()
        || value.is_undefined()
        || value.as_bool().is_some()
    {
        return Ok(value.clone());
    }

    if value.is_bigint() {
        let bigint: js_sys::BigInt = value.clone().unchecked_into();
        let bigint_str = bigint
            .to_string(10)
            .map(|s| s.into())
            .unwrap_or_else(|_| "0".to_string());
        return Ok(JsValue::from_str(&bigint_str));
    }

    // Convert Uint8Array to plain array so it deserializes as Value::Array
    // The consuming code (e.g., DataContract::from_value) will interpret these
    // as bytes based on schema
    if value.is_instance_of::<js_sys::Uint8Array>() {
        let uint8_array: js_sys::Uint8Array = value.clone().unchecked_into();
        let plain_array = js_sys::Array::from(&uint8_array);
        return Ok(plain_array.into());
    }

    if js_sys::Array::is_array(value) {
        let arr = js_sys::Array::from(value);
        let new_arr = js_sys::Array::new();
        for i in 0..arr.length() {
            let elem = arr.get(i);
            let converted = normalize_js_value_for_platform_value(&elem)?;
            new_arr.push(&converted);
        }
        return Ok(new_arr.into());
    }

    if value.is_object() && !value.is_null() {
        let obj = Object::from(value.clone());
        let new_obj = Object::new();
        let keys = Object::keys(&obj);
        for i in 0..keys.length() {
            let key = keys.get(i);
            if key.as_string().is_some() {
                let prop_value = js_sys::Reflect::get(value, &key).map_err(|e| {
                    WasmDppError::serialization(format!("Failed to get property: {:?}", e))
                })?;
                let converted = normalize_js_value_for_platform_value(&prop_value)?;
                js_sys::Reflect::set(&new_obj, &key, &converted).map_err(|e| {
                    WasmDppError::serialization(format!("Failed to set property: {:?}", e))
                })?;
            }
        }
        return Ok(new_obj.into());
    }

    // Anything else: pass through
    Ok(value.clone())
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

/// Macro to implement `toObject`, `fromObject`, `toJSON`, and `fromJSON` methods
/// for a wasm_bindgen newtype wrapper using the serde_format module.
///
/// # Usage
///
/// ```ignore
/// // For newtype wrappers: WrapperType(InnerType)
/// // JS class name defaults to Rust type name without "Wasm" suffix
/// impl_wasm_conversions!(MyTypeWasm, MyType);
/// ```
///
/// The inner type must implement `Serialize` and `DeserializeOwned`.
/// The wrapper type must implement `From<InnerType>` and have a `.0` field.
#[macro_export]
macro_rules! impl_wasm_conversions {
    // Two-argument form: wrapper type and JS class name
    ($wrapper:ty, $js_class:ident) => {
        #[wasm_bindgen::prelude::wasm_bindgen(js_class = $js_class)]
        impl $wrapper {
            #[wasm_bindgen::prelude::wasm_bindgen(js_name = toObject)]
            pub fn to_object(&self) -> Result<wasm_bindgen::JsValue, $crate::error::WasmDppError> {
                $crate::serde_format::to_object(&self.0)
            }

            #[wasm_bindgen::prelude::wasm_bindgen(js_name = fromObject)]
            pub fn from_object(obj: wasm_bindgen::JsValue) -> Result<$wrapper, $crate::error::WasmDppError> {
                $crate::serde_format::from_object(obj).map(Self)
            }

            #[wasm_bindgen::prelude::wasm_bindgen(js_name = toJSON)]
            pub fn to_json(&self) -> Result<wasm_bindgen::JsValue, $crate::error::WasmDppError> {
                $crate::serde_format::to_json(&self.0)
            }

            #[wasm_bindgen::prelude::wasm_bindgen(js_name = fromJSON)]
            pub fn from_json(js: wasm_bindgen::JsValue) -> Result<$wrapper, $crate::error::WasmDppError> {
                $crate::serde_format::from_json(js).map(Self)
            }
        }
    };
}
