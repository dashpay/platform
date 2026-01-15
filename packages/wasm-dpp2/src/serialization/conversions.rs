//! Format-aware serialization helpers for WASM.
//!
//! This module provides serialization/deserialization helpers that use serde's
//! `is_human_readable()` mechanism to determine output format:
//! - Human-readable (JSON): identifiers as Base58 strings, bytes as base64
//! - Non-human-readable (binary/WASM): identifiers as bytes → Uint8Array
//!
//! ## Main API
//!
//! For WASM types implementing `Serialize`/`Deserialize`:
//! - [`to_json`] / [`from_json`] - Human-readable format (base58 identifiers, base64 bytes)
//! - [`to_object`] / [`from_object`] - Binary format (Uint8Array for bytes, BigInt for u64)
//!
//! For converting between JsValue and Rust types:
//! - [`js_value_to_json`] - JsValue → serde_json::Value (handles BigInt, WASM objects)
//! - [`json_to_js_value`] - serde_json::Value → JsValue
//! - [`js_value_to_platform_value`] - JsValue → platform_value::Value
//!
//! For platform_value specifically:
//! - [`platform_value_to_object`] / [`platform_value_from_object`] - Binary format
//! - [`platform_value_to_json`] - Human-readable format

use crate::error::{WasmDppError, WasmDppResult};
use dpp::platform_value;
use js_sys::Object;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value as JsonValue;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;

/// Try to call toJSON() on a WASM object if it has one.
///
/// Returns Some(result) if the object has a toJSON method and it succeeds,
/// None otherwise (for plain objects, arrays, primitives, etc.)
fn try_call_to_json(value: &JsValue) -> Option<JsValue> {
    if !value.is_object() || value.is_null() || js_sys::Array::is_array(value) {
        return None;
    }

    // Check for toJSON method
    let to_json_fn = js_sys::Reflect::get(value, &JsValue::from_str("toJSON")).ok()?;
    if !to_json_fn.is_function() {
        return None;
    }

    let func: js_sys::Function = to_json_fn.unchecked_into();
    func.call0(value).ok()
}

/// Convert JsValue to serde_json::Value, handling BigInt values and WASM objects.
///
/// This function:
/// - Converts BigInt values to strings (JSON doesn't support BigInt natively)
/// - For WASM objects with a `toJSON` method, calls that method first to get proper JSON
/// - Falls back to serde_wasm_bindgen conversion for plain objects
pub fn js_value_to_json(value: &JsValue) -> WasmDppResult<JsonValue> {
    // Check if the value has a toJSON method (WASM objects like DataContractWasm, IdentityWasm)
    if let Some(json_result) = try_call_to_json(value) {
        // Recursively convert the result (it might contain BigInt or nested WASM objects)
        return js_value_to_json(&json_result);
    }

    let normalized = normalize_js_value_for_json(value)?;
    serde_wasm_bindgen::from_value(normalized).map_err(|e| {
        WasmDppError::serialization(format!("Failed to convert JsValue to JSON: {}", e))
    })
}

/// Convert serde_json::Value to JsValue using JSON-compatible serialization.
///
/// This ensures objects become plain JS objects (not Maps).
pub fn json_to_js_value(value: &JsonValue) -> WasmDppResult<JsValue> {
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    value.serialize(&serializer).map_err(|e| {
        WasmDppError::serialization(format!("Failed to convert JSON to JsValue: {}", e))
    })
}

/// Recursively normalizes a JsValue for JSON conversion.
///
/// This converts:
/// - WASM objects with toJSON() method - calls toJSON() and normalizes the result
/// - BigInt values to strings (JSON doesn't support BigInt natively)
/// - Uint8Array to plain arrays (so they serialize as JSON number arrays)
/// - JavaScript Map to plain objects
/// - Recursively processes nested objects and arrays
///
/// Performance: Uses fast path for primitives, only recursively processes objects/arrays.
fn normalize_js_value_for_json(value: &JsValue) -> WasmDppResult<JsValue> {
    // Fast path: primitives that can't contain BigInt or need conversion
    if value.is_string()
        || value.as_f64().is_some()
        || value.is_null()
        || value.is_undefined()
        || value.as_bool().is_some()
    {
        return Ok(value.clone());
    }

    // Convert BigInt to string (JSON doesn't support BigInt)
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

    // Convert JavaScript Map to an object for JSON compatibility
    // Maps don't have enumerable properties, so Object.keys() returns empty array
    if value.is_instance_of::<js_sys::Map>() {
        return normalize_map_for_json(value);
    }

    // Handle arrays - recursively normalize each element
    if js_sys::Array::is_array(value) {
        let arr = js_sys::Array::from(value);
        let new_arr = js_sys::Array::new();
        for i in 0..arr.length() {
            let elem = arr.get(i);
            let normalized = normalize_js_value_for_json(&elem)?;
            new_arr.push(&normalized);
        }
        return Ok(new_arr.into());
    }

    // Handle objects - check for toJSON method first (WASM objects), then normalize properties
    if value.is_object() && !value.is_null() {
        // Try to call toJSON() on WASM objects (Identity, Identifier, DataContract, etc.)
        if let Some(json_result) = try_call_to_json(value) {
            // Recursively normalize the result (might contain BigInt, nested objects, etc.)
            return normalize_js_value_for_json(&json_result);
        }

        // Plain object - normalize each property
        let obj = Object::from(value.clone());
        let new_obj = Object::new();
        let keys = Object::keys(&obj);
        for i in 0..keys.length() {
            let key = keys.get(i);
            if key.as_string().is_some() {
                let prop_value = js_sys::Reflect::get(value, &key).map_err(|e| {
                    WasmDppError::serialization(format!("Failed to get property: {:?}", e))
                })?;
                let normalized = normalize_js_value_for_json(&prop_value)?;
                js_sys::Reflect::set(&new_obj, &key, &normalized).map_err(|e| {
                    WasmDppError::serialization(format!("Failed to set property: {:?}", e))
                })?;
            }
        }
        return Ok(new_obj.into());
    }

    // Anything else (Symbol, Function, etc.): pass through
    Ok(value.clone())
}

/// Convert a JavaScript Map key to a string for JSON object keys.
fn map_key_to_string(key: &JsValue) -> String {
    // String keys - use as-is
    if let Some(s) = key.as_string() {
        return s;
    }

    // Number keys - convert to string
    if key.as_f64().is_some() {
        return js_sys::Number::from(key.clone())
            .to_string(10)
            .map(|s| s.into())
            .unwrap_or_else(|_| "0".to_string());
    }

    // BigInt keys - convert to string
    if key.is_bigint() {
        let bigint: js_sys::BigInt = key.clone().unchecked_into();
        return bigint
            .to_string(10)
            .map(|s| s.into())
            .unwrap_or_else(|_| "0".to_string());
    }

    // Objects with toString (like Identifier) - call toString()
    if let Ok(to_string_fn) = js_sys::Reflect::get(key, &JsValue::from_str("toString"))
        && to_string_fn.is_function()
    {
        let func: js_sys::Function = to_string_fn.unchecked_into();
        if let Ok(str_result) = func.call0(key) {
            if let Some(s) = str_result.as_string() {
                return s;
            }
        }
    }

    // Fallback - use debug representation
    format!("{:?}", key)
}

/// Convert a JavaScript Map to a plain object for JSON serialization.
fn normalize_map_for_json(value: &JsValue) -> WasmDppResult<JsValue> {
    let map: js_sys::Map = value.clone().unchecked_into();
    let new_obj = Object::new();

    // We need to collect errors from the closure since for_each doesn't support Result
    let error: std::cell::RefCell<Option<WasmDppError>> = std::cell::RefCell::new(None);

    map.for_each(&mut |val, key| {
        // Skip if we already have an error
        if error.borrow().is_some() {
            return;
        }

        let key_str = map_key_to_string(&key);

        // Normalize the value - handle WASM objects, BigInt, nested Maps, etc.
        match normalize_js_value_for_json(&val) {
            Ok(normalized_val) => {
                if let Err(e) = js_sys::Reflect::set(&new_obj, &JsValue::from_str(&key_str), &normalized_val) {
                    *error.borrow_mut() = Some(WasmDppError::serialization(format!(
                        "Failed to set Map entry '{}': {:?}",
                        key_str, e
                    )));
                }
            }
            Err(e) => {
                *error.borrow_mut() = Some(e);
            }
        }
    });

    // Check if any error occurred during iteration
    if let Some(e) = error.into_inner() {
        return Err(e);
    }

    Ok(new_obj.into())
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
    let serializer =
        serde_wasm_bindgen::Serializer::json_compatible().serialize_human_readable(true);
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

/// Serialize platform_value::Value to JsValue as JSON-compatible (human-readable).
///
/// Converts Value::Identifier and Value::Bytes to base58/base64 strings for JSON compatibility.
pub fn platform_value_to_json(value: &platform_value::Value) -> WasmDppResult<JsValue> {
    let converted = convert_value_for_json(value);
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    converted
        .serialize(&serializer)
        .map_err(|e| WasmDppError::serialization(format!("platform_value_to_json: {}", e)))
}

/// Convert platform_value::Value for JSON serialization.
/// Transforms binary types to their string representations.
fn convert_value_for_json(value: &platform_value::Value) -> platform_value::Value {
    use dpp::platform_value::string_encoding::{Encoding, encode};

    match value {
        platform_value::Value::Identifier(bytes) => {
            platform_value::Value::Text(encode(bytes, Encoding::Base58))
        }
        platform_value::Value::Bytes(bytes) => {
            platform_value::Value::Text(encode(bytes, Encoding::Base64))
        }
        platform_value::Value::Bytes20(bytes) => {
            platform_value::Value::Text(encode(bytes, Encoding::Base64))
        }
        platform_value::Value::Bytes32(bytes) => {
            platform_value::Value::Text(encode(bytes, Encoding::Base64))
        }
        platform_value::Value::Bytes36(bytes) => {
            platform_value::Value::Text(encode(bytes, Encoding::Base64))
        }
        platform_value::Value::Map(map) => platform_value::Value::Map(
            map.iter()
                .map(|(k, v)| (convert_value_for_json(k), convert_value_for_json(v)))
                .collect(),
        ),
        platform_value::Value::Array(arr) => {
            platform_value::Value::Array(arr.iter().map(convert_value_for_json).collect())
        }
        other => other.clone(),
    }
}

/// Deserialize JsValue to platform_value::Value.
///
/// serde-wasm-bindgen's deserialize_any handles Uint8Array via visit_byte_buf, which creates
/// Value::Bytes. BigInt is handled via visit_i64/visit_u64.
pub fn platform_value_from_object(value: JsValue) -> WasmDppResult<platform_value::Value> {
    serde_wasm_bindgen::from_value(value)
        .map_err(|e| WasmDppError::serialization(format!("platform_value_from_object: {}", e)))
}

/// Convert a JsValue to platform_value::Value, properly handling BigInt and Uint8Array.
pub fn js_value_to_platform_value(value: &JsValue) -> WasmDppResult<platform_value::Value> {
    // Null
    if value.is_null() || value.is_undefined() {
        return Ok(platform_value::Value::Null);
    }

    // Boolean
    if let Some(b) = value.as_bool() {
        return Ok(platform_value::Value::Bool(b));
    }

    // Number (f64) - try to convert to integer if it's a whole number
    if let Some(num) = value.as_f64() {
        // Check if it's a whole number that fits in i64/u64
        if num.fract() == 0.0 {
            if num >= 0.0 && num <= u64::MAX as f64 {
                return Ok(platform_value::Value::U64(num as u64));
            } else if num >= i64::MIN as f64 && num < 0.0 {
                return Ok(platform_value::Value::I64(num as i64));
            }
        }
        return Ok(platform_value::Value::Float(num));
    }

    // BigInt - convert to appropriate integer type
    if value.is_bigint() {
        let bigint: js_sys::BigInt = value.clone().unchecked_into();
        // Get string representation and parse
        let bigint_str: String = bigint
            .to_string(10)
            .map(|s| s.into())
            .unwrap_or_else(|_| "0".to_string());

        // Try parsing as u64 first (most common for revision, timestamps, etc.)
        if let Ok(val) = bigint_str.parse::<u64>() {
            return Ok(platform_value::Value::U64(val));
        }
        // Try i64 for negative values
        if let Ok(val) = bigint_str.parse::<i64>() {
            return Ok(platform_value::Value::I64(val));
        }
        // Try u128/i128 for larger values
        if let Ok(val) = bigint_str.parse::<u128>() {
            return Ok(platform_value::Value::U128(val));
        }
        if let Ok(val) = bigint_str.parse::<i128>() {
            return Ok(platform_value::Value::I128(val));
        }
        // Fall back to string if it doesn't fit
        return Ok(platform_value::Value::Text(bigint_str));
    }

    // String
    if let Some(s) = value.as_string() {
        return Ok(platform_value::Value::Text(s));
    }

    // Uint8Array - convert to bytes
    if value.is_instance_of::<js_sys::Uint8Array>() {
        let uint8_array: js_sys::Uint8Array = value.clone().unchecked_into();
        let bytes = uint8_array.to_vec();
        // Check for identifier (32 bytes)
        if bytes.len() == 32 {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            return Ok(platform_value::Value::Identifier(arr));
        }
        return Ok(platform_value::Value::Bytes(bytes));
    }

    // Array
    if js_sys::Array::is_array(value) {
        let arr = js_sys::Array::from(value);
        let mut result = Vec::new();
        for i in 0..arr.length() {
            let elem = arr.get(i);
            result.push(js_value_to_platform_value(&elem)?);
        }
        return Ok(platform_value::Value::Array(result));
    }

    // Object (Map)
    if value.is_object() {
        let obj = js_sys::Object::from(value.clone());
        let keys = js_sys::Object::keys(&obj);
        let mut map = Vec::new();
        for i in 0..keys.length() {
            let key = keys.get(i);
            if let Some(key_str) = key.as_string() {
                let prop_value = js_sys::Reflect::get(value, &key).map_err(|e| {
                    WasmDppError::serialization(format!("Failed to get property: {:?}", e))
                })?;
                let converted = js_value_to_platform_value(&prop_value)?;
                map.push((platform_value::Value::Text(key_str), converted));
            }
        }
        return Ok(platform_value::Value::Map(map));
    }

    // Fallback
    Err(WasmDppError::serialization(
        "Unsupported JsValue type for platform_value conversion".to_string(),
    ))
}

/// Test helper: Convert a JsValue to a JSON-compatible JsValue.
///
/// This function is exposed for testing purposes to validate the Map normalization
/// and BigInt handling logic in unit tests. It calls `js_value_to_json` internally
/// and converts the result back to a JsValue.
///
/// # Example (JavaScript)
/// ```javascript
/// const map = new Map([['key', 'value']]);
/// const json = testJsValueToJson(map);
/// console.log(json); // { key: 'value' }
/// ```
#[wasm_bindgen(js_name = testJsValueToJson)]
pub fn test_js_value_to_json(value: &JsValue) -> Result<JsValue, WasmDppError> {
    let json_value = js_value_to_json(value)?;
    json_to_js_value(&json_value)
}

/// Macro to implement `toObject`, `fromObject`, `toJSON`, and `fromJSON` methods
/// for a wasm_bindgen newtype wrapper using the serialization::conversions module.
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
                $crate::serialization::conversions::to_object(&self.0)
            }

            #[wasm_bindgen::prelude::wasm_bindgen(js_name = fromObject)]
            pub fn from_object(
                obj: wasm_bindgen::JsValue,
            ) -> Result<$wrapper, $crate::error::WasmDppError> {
                $crate::serialization::conversions::from_object(obj).map(Self)
            }

            #[wasm_bindgen::prelude::wasm_bindgen(js_name = toJSON)]
            pub fn to_json(&self) -> Result<wasm_bindgen::JsValue, $crate::error::WasmDppError> {
                $crate::serialization::conversions::to_json(&self.0)
            }

            #[wasm_bindgen::prelude::wasm_bindgen(js_name = fromJSON)]
            pub fn from_json(
                js: wasm_bindgen::JsValue,
            ) -> Result<$wrapper, $crate::error::WasmDppError> {
                $crate::serialization::conversions::from_json(js).map(Self)
            }
        }
    };
}
