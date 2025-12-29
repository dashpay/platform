use crate::WasmSdkError;
use platform_value::Value;
use wasm_bindgen::JsValue;
use wasm_dpp2::serialization::conversions;

/// Convert a `JsValue` coming from JavaScript into a Platform `Value`.
///
/// Uses object-format conversion which properly handles BigInt and Uint8Array.
pub fn js_value_to_platform_value(value: JsValue) -> Result<Value, WasmSdkError> {
    conversions::js_value_to_platform_value(&value).map_err(|err| {
        WasmSdkError::invalid_argument(format!(
            "Failed to convert JS value to platform value: {err}"
        ))
    })
}

/// Convert an iterable collection of `JsValue` into Platform `Value`s.
pub fn js_values_to_platform_values<I>(values: I) -> Result<Vec<Value>, WasmSdkError>
where
    I: IntoIterator<Item = JsValue>,
{
    values.into_iter().map(js_value_to_platform_value).collect()
}
