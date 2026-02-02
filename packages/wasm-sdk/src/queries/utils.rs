use dash_sdk::dpp::platform_value::Value as PlatformValue;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value as JsonValue;
use wasm_bindgen::JsValue;
use wasm_dpp2::serialization::conversions::{from_object, js_value_to_platform_value};

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

    from_object(value)
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid {}: {}", context, e)))
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

    from_object(value)
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid {}: {}", context, e)))
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
    values
        .unwrap_or_default()
        .into_iter()
        .map(|value| {
            // Use json_compatible() to ensure objects become plain JS objects (not Maps)
            let serializer = serde_wasm_bindgen::Serializer::json_compatible();
            let js_value = value.serialize(&serializer).map_err(|err| {
                WasmSdkError::invalid_argument(format!("Invalid {} entry: {}", field_name, err))
            })?;
            js_value_to_platform_value(&js_value).map_err(|err| {
                WasmSdkError::invalid_argument(format!("Invalid {} entry: {}", field_name, err))
            })
        })
        .collect()
}
