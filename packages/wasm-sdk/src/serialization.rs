use crate::WasmSdkError;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value as JsonValue;
use wasm_bindgen::JsValue;

pub fn to_object<T: Serialize>(value: &T) -> Result<JsValue, WasmSdkError> {
    serde_wasm_bindgen::to_value(value).map_err(|e| WasmSdkError::serialization(&e.to_string()))
}

pub fn from_object<T: DeserializeOwned>(value: JsValue) -> Result<T, WasmSdkError> {
    serde_wasm_bindgen::from_value(value).map_err(|e| WasmSdkError::serialization(&e.to_string()))
}

pub fn to_json_value<T: Serialize>(value: &T) -> Result<JsValue, WasmSdkError> {
    let json =
        serde_json::to_value(value).map_err(|e| WasmSdkError::serialization(&e.to_string()))?;
    serde_wasm_bindgen::to_value(&json).map_err(|e| WasmSdkError::serialization(&e.to_string()))
}

pub fn from_json_value<T: DeserializeOwned>(value: JsValue) -> Result<T, WasmSdkError> {
    let json: JsonValue = serde_wasm_bindgen::from_value(value)
        .map_err(|e| WasmSdkError::serialization(&e.to_string()))?;
    serde_json::from_value(json).map_err(|e| WasmSdkError::serialization(&e.to_string()))
}

pub fn js_to_json_value(value: JsValue) -> Result<JsonValue, WasmSdkError> {
    serde_wasm_bindgen::from_value(value).map_err(|e| WasmSdkError::serialization(&e.to_string()))
}

pub fn json_value_to_js(value: &JsonValue) -> Result<JsValue, WasmSdkError> {
    serde_wasm_bindgen::to_value(value).map_err(|e| WasmSdkError::serialization(&e.to_string()))
}

#[macro_export]
macro_rules! impl_wasm_object_json {
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
}
