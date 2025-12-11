use crate::WasmSdkError;
use platform_value::Value;
use wasm_bindgen::JsValue;
use wasm_dpp2::utils::ToSerdeJSONExt;

/// Convert a `JsValue` coming from JavaScript into a Platform `Value`.
pub fn js_value_to_platform_value(value: JsValue) -> Result<Value, WasmSdkError> {
    value.with_serde_to_platform_value().map_err(|err| {
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
