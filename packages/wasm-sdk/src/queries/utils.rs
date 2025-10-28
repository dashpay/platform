use dash_sdk::dpp::platform_value::{
    string_encoding::Encoding, Identifier, Value as PlatformValue,
};
use serde::de::DeserializeOwned;
use serde_json::Value as JsonValue;
use wasm_bindgen::JsValue;

use crate::utils::{js_value_to_platform_value, js_values_to_platform_values};
use crate::WasmSdkError;

pub(crate) fn deserialize_required_query<T, Q>(
    query: Q,
    missing_error: &str,
    context: &str,
) -> Result<T, WasmSdkError>
where
    T: DeserializeOwned,
    Q: Into<JsValue>,
{
    let value = query.into();

    if value.is_null() || value.is_undefined() {
        return Err(WasmSdkError::invalid_argument(missing_error.to_string()));
    }

    serde_wasm_bindgen::from_value(value)
        .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid {}: {}", context, err)))
}

pub(crate) fn deserialize_query_with_default<T, Q>(
    query: Option<Q>,
    context: &str,
) -> Result<T, WasmSdkError>
where
    T: Default + DeserializeOwned,
    Q: Into<JsValue>,
{
    let value = query.map(Into::into).unwrap_or_else(|| JsValue::UNDEFINED);

    if value.is_null() || value.is_undefined() {
        return Ok(T::default());
    }

    serde_wasm_bindgen::from_value(value)
        .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid {}: {}", context, err)))
}

pub(crate) fn convert_optional_limit(
    limit: Option<u32>,
    field: &str,
) -> Result<Option<u16>, WasmSdkError> {
    match limit {
        Some(0) => Ok(None),
        Some(value) => {
            if value > u16::MAX as u32 {
                Err(WasmSdkError::invalid_argument(format!(
                    "{} {} exceeds maximum of {}",
                    field,
                    value,
                    u16::MAX
                )))
            } else {
                Ok(Some(value as u16))
            }
        }
        None => Ok(None),
    }
}

pub(crate) fn convert_json_values_to_platform_values(
    values: Option<Vec<JsonValue>>,
    field_name: &str,
) -> Result<Vec<PlatformValue>, WasmSdkError> {
    let js_values = values
        .unwrap_or_default()
        .into_iter()
        .map(|value| {
            serde_wasm_bindgen::to_value(&value).map_err(|err| {
                WasmSdkError::invalid_argument(format!("Invalid {} entry: {}", field_name, err))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    js_values_to_platform_values(js_values)
}

pub(crate) fn convert_json_value_to_platform_value(
    value: JsonValue,
    field_name: &str,
) -> Result<PlatformValue, WasmSdkError> {
    let js_value = serde_wasm_bindgen::to_value(&value).map_err(|err| {
        WasmSdkError::invalid_argument(format!("Invalid {} entry: {}", field_name, err))
    })?;

    js_value_to_platform_value(js_value)
}

pub(crate) fn identifier_from_base58(value: &str, field: &str) -> Result<Identifier, WasmSdkError> {
    Identifier::from_string(value, Encoding::Base58)
        .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid {}: {}", field, err)))
}
