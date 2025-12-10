use crate::WasmSdkError;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value as JsonValue;
use wasm_bindgen::JsValue;
use wasm_dpp2::serde_format;

/// Serialize to JsValue with WASM format context.
pub fn to_object<T: Serialize>(value: &T) -> Result<JsValue, WasmSdkError> {
    serde_format::to_wasm_value(value).map_err(|e| WasmSdkError::serialization(&e.to_string()))
}

/// Deserialize from JsValue with WASM format context.
pub fn from_object<T: DeserializeOwned>(value: JsValue) -> Result<T, WasmSdkError> {
    serde_format::from_wasm_value(value).map_err(|e| WasmSdkError::serialization(&e.to_string()))
}

/// Serialize to JsValue with JSON format context.
pub fn to_json_value<T: Serialize>(value: &T) -> Result<JsValue, WasmSdkError> {
    serde_format::to_js_value_json(value).map_err(WasmSdkError::from)
}

/// Deserialize from JsValue with JSON format context.
pub fn from_json_value<T: DeserializeOwned>(value: JsValue) -> Result<T, WasmSdkError> {
    serde_format::from_js_value_json(value).map_err(WasmSdkError::from)
}

/// Convert JsValue to serde_json::Value.
pub fn js_to_json_value(value: JsValue) -> Result<JsonValue, WasmSdkError> {
    serde_format::js_value_to_json(&value).map_err(WasmSdkError::from)
}

/// Convert serde_json::Value to JsValue.
pub fn json_value_to_js(value: &JsonValue) -> Result<JsValue, WasmSdkError> {
    serde_format::json_to_js_value(value).map_err(WasmSdkError::from)
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
