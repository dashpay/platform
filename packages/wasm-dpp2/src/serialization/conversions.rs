//! Format-aware serialization helpers for WASM.
//!
//! This module provides serialization/deserialization helpers that use serde's
//! `is_human_readable()` mechanism to determine output format:
//! - Human-readable (JSON): identifiers as Base58 strings, bytes as base64
//! - Non-human-readable (binary/WASM): identifiers as bytes → Uint8Array

use crate::error::{WasmDppError, WasmDppResult};
use dpp::platform_value;
use js_sys::Object;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value as JsonValue;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

/// Serialize a value to `serde_json::Value`.
///
/// serde_json's serializer has `is_human_readable() -> true`, so types like
/// IdentifierWasm will serialize as Base58 strings.
pub fn to_json_value<T: Serialize>(value: &T) -> Result<JsonValue, serde_json::Error> {
    serde_json::to_value(value)
}

/// Deserialize from `serde_json::Value`.
pub fn from_json_value<T: DeserializeOwned>(value: JsonValue) -> Result<T, serde_json::Error> {
    serde_json::from_value(value)
}

/// Serialize a value to JSON string.
pub fn to_json_string<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    serde_json::to_string(value)
}

/// Serialize a value to pretty-printed JSON string.
pub fn to_json_string_pretty<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(value)
}

/// Deserialize from JSON string.
pub fn from_json_str<T: DeserializeOwned>(s: &str) -> Result<T, serde_json::Error> {
    serde_json::from_str(s)
}

/// Serialize a value to `JsValue` using serde-wasm-bindgen.
///
/// serde-wasm-bindgen's default serializer has `is_human_readable() -> false`,
/// so types like IdentifierWasm will serialize as bytes → Uint8Array.
pub fn to_wasm_value<T: Serialize>(value: &T) -> Result<JsValue, serde_wasm_bindgen::Error> {
    serde_wasm_bindgen::to_value(value)
}

/// Deserialize from `JsValue` using serde-wasm-bindgen.
pub fn from_wasm_value<T: DeserializeOwned>(js: JsValue) -> Result<T, serde_wasm_bindgen::Error> {
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

/// Convert JsValue to serde_json::Value, handling BigInt values and WASM objects.
///
/// This function:
/// - Converts BigInt values to strings (JSON doesn't support BigInt natively)
/// - For WASM objects with a `toJSON` method, calls that method first to get proper JSON
/// - Falls back to serde_wasm_bindgen conversion for plain objects
pub fn js_value_to_json(value: &JsValue) -> WasmDppResult<JsonValue> {
    // Check if the value has a toJSON method (WASM objects like DataContractWasm, IdentityWasm)
    if value.is_object() && !value.is_null() && !js_sys::Array::is_array(value) {
        if let Ok(to_json_fn) = js_sys::Reflect::get(value, &JsValue::from_str("toJSON")) {
            if to_json_fn.is_function() {
                let func: js_sys::Function = to_json_fn.unchecked_into();
                // Call toJSON() on the object
                if let Ok(json_result) = func.call0(value) {
                    // Recursively convert the result (it might contain BigInt or nested WASM objects)
                    return js_value_to_json(&json_result);
                }
            }
        }
    }

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
    use dpp::platform_value::string_encoding::{encode, Encoding};

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
            pub fn from_object(obj: wasm_bindgen::JsValue) -> Result<$wrapper, $crate::error::WasmDppError> {
                $crate::serialization::conversions::from_object(obj).map(Self)
            }

            #[wasm_bindgen::prelude::wasm_bindgen(js_name = toJSON)]
            pub fn to_json(&self) -> Result<wasm_bindgen::JsValue, $crate::error::WasmDppError> {
                $crate::serialization::conversions::to_json(&self.0)
            }

            #[wasm_bindgen::prelude::wasm_bindgen(js_name = fromJSON)]
            pub fn from_json(js: wasm_bindgen::JsValue) -> Result<$wrapper, $crate::error::WasmDppError> {
                $crate::serialization::conversions::from_json(js).map(Self)
            }
        }
    };
}
