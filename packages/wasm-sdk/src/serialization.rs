use crate::WasmSdkError;
use js_sys::Object;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value as JsonValue;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use wasm_dpp2::serde_format;

/// Serialize to JsValue with WASM format context.
///
/// Types like `IdentifierWasm` will serialize as bytes (Uint8Array).
pub fn to_object<T: Serialize>(value: &T) -> Result<JsValue, WasmSdkError> {
    serde_format::to_wasm_value(value).map_err(|e| WasmSdkError::serialization(&e.to_string()))
}

/// Deserialize from JsValue with WASM format context.
pub fn from_object<T: DeserializeOwned>(value: JsValue) -> Result<T, WasmSdkError> {
    serde_format::from_wasm_value(value).map_err(|e| WasmSdkError::serialization(&e.to_string()))
}

/// Serialize to JsValue with JSON format context.
///
/// Types like `IdentifierWasm` will serialize as Base58 strings.
/// The result is a JSON-compatible JsValue (strings instead of Uint8Array for identifiers).
pub fn to_json_value<T: Serialize>(value: &T) -> Result<JsValue, WasmSdkError> {
    let json = serde_format::to_json_value(value)
        .map_err(|e| WasmSdkError::serialization(&e.to_string()))?;

    // Use json_compatible() serializer to convert serde_json::Value to JsValue
    // This ensures proper JSON semantics (human_readable = true)
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    json.serialize(&serializer)
        .map_err(|e| WasmSdkError::serialization(&e.to_string()))
}

/// Deserialize from JsValue with JSON format context.
///
/// This function handles BigInt values by converting them to strings first, which is consistent
/// with how we serialize large integers to JSON.
pub fn from_json_value<T: DeserializeOwned>(value: JsValue) -> Result<T, WasmSdkError> {
    // Convert BigInts to strings first to avoid conversion errors
    let converted = convert_bigints_to_strings(&value)?;
    // Convert JsValue to serde_json::Value
    let json: JsonValue = serde_wasm_bindgen::from_value(converted)
        .map_err(|e| WasmSdkError::serialization(&e.to_string()))?;
    // Then deserialize with JSON format context
    serde_format::from_json_value(json).map_err(|e| WasmSdkError::serialization(&e.to_string()))
}

/// Convert JsValue to serde_json::Value.
///
/// This function handles BigInt values by converting them to strings, which is the standard
/// approach for serializing large integers in JSON (since JSON numbers are limited to 53-bit
/// precision in JavaScript).
pub fn js_to_json_value(value: JsValue) -> Result<JsonValue, WasmSdkError> {
    let converted = convert_bigints_to_strings(&value)?;
    serde_wasm_bindgen::from_value(converted)
        .map_err(|e| WasmSdkError::serialization(&e.to_string()))
}

/// Recursively converts BigInt values to strings in a JsValue.
///
/// This is necessary because serde_wasm_bindgen cannot convert BigInt values larger than
/// Number.MAX_SAFE_INTEGER to JSON numbers.
pub fn convert_bigints_to_strings(value: &JsValue) -> Result<JsValue, WasmSdkError> {
    if value.is_bigint() {
        // Convert BigInt to string using js_sys::BigInt
        let bigint: js_sys::BigInt = value.clone().unchecked_into();
        let bigint_str = bigint.to_string(10)
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
                let prop_value = js_sys::Reflect::get(value, &key)
                    .map_err(|e| WasmSdkError::serialization(&format!("Failed to get property: {:?}", e)))?;
                let converted = convert_bigints_to_strings(&prop_value)?;
                js_sys::Reflect::set(&new_obj, &key, &converted)
                    .map_err(|e| WasmSdkError::serialization(&format!("Failed to set property: {:?}", e)))?;
            }
        }
        Ok(new_obj.into())
    } else {
        // Return primitive values as-is
        Ok(value.clone())
    }
}

/// Convert serde_json::Value to JsValue.
///
/// Uses JSON-compatible serialization so objects become plain JS objects (not Maps).
pub fn json_value_to_js(value: &JsonValue) -> Result<JsValue, WasmSdkError> {
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    value
        .serialize(&serializer)
        .map_err(|e| WasmSdkError::serialization(&e.to_string()))
}

/// Serialize to JsValue with WASM format context, keeping bytes as Uint8Array.
///
/// This is an alias for `to_object` - both use WASM format context which
/// serializes byte fields as Uint8Array rather than strings.
pub fn to_object_bytes<T: Serialize>(value: &T) -> Result<JsValue, WasmSdkError> {
    to_object(value)
}

#[macro_export]
macro_rules! impl_wasm_object_json {
    // Single-argument form: Rust type name equals JS class name
    ($ty:ty) => {
        #[wasm_bindgen]
        impl $ty {
            #[wasm_bindgen(js_name = toObject)]
            pub fn to_object(&self) -> Result<wasm_bindgen::JsValue, $crate::WasmSdkError> {
                $crate::serialization::to_object(self)
            }

            #[wasm_bindgen(js_name = fromObject)]
            pub fn from_object(obj: wasm_bindgen::JsValue) -> Result<$ty, $crate::WasmSdkError> {
                $crate::serialization::from_object(obj)
            }

            #[wasm_bindgen(js_name = toJSON)]
            pub fn to_json(&self) -> Result<wasm_bindgen::JsValue, $crate::WasmSdkError> {
                $crate::serialization::to_json_value(self)
            }

            #[wasm_bindgen(js_name = fromJSON)]
            pub fn from_json(js: wasm_bindgen::JsValue) -> Result<$ty, $crate::WasmSdkError> {
                $crate::serialization::from_json_value(js)
            }
        }
    };
    // Two-argument form: second argument is the JS class name
    ($ty:ty, $js_class:ident) => {
        #[wasm_bindgen(js_class = $js_class)]
        impl $ty {
            #[wasm_bindgen(js_name = toObject)]
            pub fn to_object(&self) -> Result<wasm_bindgen::JsValue, $crate::WasmSdkError> {
                $crate::serialization::to_object(self)
            }

            #[wasm_bindgen(js_name = fromObject)]
            pub fn from_object(obj: wasm_bindgen::JsValue) -> Result<$ty, $crate::WasmSdkError> {
                $crate::serialization::from_object(obj)
            }

            #[wasm_bindgen(js_name = toJSON)]
            pub fn to_json(&self) -> Result<wasm_bindgen::JsValue, $crate::WasmSdkError> {
                $crate::serialization::to_json_value(self)
            }

            #[wasm_bindgen(js_name = fromJSON)]
            pub fn from_json(js: wasm_bindgen::JsValue) -> Result<$ty, $crate::WasmSdkError> {
                $crate::serialization::from_json_value(js)
            }
        }
    };
}
