use dash_sdk::dpp::platform_value::{self, Value as PlatformValue};
use js_sys::{Function, Object, Reflect, Uint8Array};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value as JsonValue;
use std::cell::RefCell;
use wasm_bindgen::{JsCast, JsValue};
use wasm_dpp2::serialization::conversions::{
    from_object, js_value_to_platform_value, platform_value_from_object,
};

use crate::WasmSdkError;
use dash_sdk::platform::Identifier;
use wasm_dpp2::identifier::{IdentifierLikeJs, IdentifierWasm};

/// The contract a query names, from any `IdentifierLike`.
pub(crate) fn parse_contract_id(id: IdentifierLikeJs) -> Result<Identifier, WasmSdkError> {
    id.try_into()
        .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid contract id: {err}")))
}

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

#[derive(Clone)]
struct IdentifierClass {
    prototype: Object,
    to_bytes: Function,
    is_prototype_of: Function,
}

thread_local! {
    static IDENTIFIER_CLASS: RefCell<Option<IdentifierClass>> = const { RefCell::new(None) };
}

fn identifier_class() -> Option<IdentifierClass> {
    if let Some(class) = IDENTIFIER_CLASS.with(|cached| cached.borrow().clone()) {
        return Some(class);
    }

    let temporary: JsValue = IdentifierWasm::from([0; 32]).into();
    let free = Reflect::get(&temporary, &JsValue::from_str("free"))
        .and_then(|value| value.dyn_into::<Function>());
    let captured = (|| {
        let prototype = Reflect::get_prototype_of(&temporary)?;
        let to_bytes =
            Reflect::get(&prototype, &JsValue::from_str("toBytes"))?.dyn_into::<Function>()?;
        let object_prototype = Reflect::get_prototype_of(&Object::new())?;
        let is_prototype_of = Reflect::get(&object_prototype, &JsValue::from_str("isPrototypeOf"))?
            .dyn_into::<Function>()?;
        Ok::<_, JsValue>(IdentifierClass {
            prototype,
            to_bytes,
            is_prototype_of,
        })
    })();
    // Free the trusted temporary even when capturing its class fails. Inputs are borrowed.
    let cleanup = free.and_then(|function| function.call0(&temporary));
    let class = match (captured, cleanup) {
        (Ok(class), Ok(_)) => class,
        _ => return None,
    };
    IDENTIFIER_CLASS.with(|cached| *cached.borrow_mut() = Some(class.clone()));
    Some(class)
}

fn local_identifier_bytes(value: &JsValue) -> Option<[u8; 32]> {
    // Clone cached handles before any JavaScript call, which may reenter the parser.
    let class = identifier_class()?;
    if class
        .is_prototype_of
        .call1(&class.prototype, value)
        .ok()?
        .as_bool()
        != Some(true)
    {
        return None;
    }

    // This detects freed/moved wrappers without invoking an input's pointer getter.
    // The class check is nominal, as for generated typed arguments; it does not attest
    // allocation ownership against arbitrary prototype or pointer mutation.
    let descriptor = Reflect::get_own_property_descriptor(
        &Object::from(value.clone()),
        &JsValue::from_str("__wbg_ptr"),
    )
    .ok()?;
    let pointer = Reflect::get(&descriptor, &JsValue::from_str("value"))
        .ok()?
        .as_f64()?;
    if !pointer.is_finite() || pointer <= 0.0 || pointer > u32::MAX as f64 || pointer.fract() != 0.0
    {
        return None;
    }

    // Use the captured base method, never an override supplied by the query value.
    let bytes = class
        .to_bytes
        .call0(value)
        .ok()?
        .dyn_into::<Uint8Array>()
        .ok()?;
    if bytes.length() != 32 {
        return None;
    }
    bytes.to_vec().try_into().ok()
}

fn normalize_identifier(value: &mut PlatformValue, original: &JsValue) {
    if matches!(value, PlatformValue::Map(_)) {
        if let Some(bytes) = local_identifier_bytes(original) {
            *value = PlatformValue::Identifier(bytes);
        }
    }
}

fn normalize_identifier_path(value: &mut PlatformValue, original: &JsValue, path: &[&str]) {
    let Some((field, remaining)) = path.split_first() else {
        return;
    };
    let PlatformValue::Map(entries) = value else {
        return;
    };
    let Some((_, node)) = entries
        .iter_mut()
        .find(|(key, _)| matches!(key, PlatformValue::Text(text) if text.as_str() == *field))
    else {
        return;
    };
    let Ok(original) = Reflect::get(original, &JsValue::from_str(field)) else {
        return;
    };
    if remaining.is_empty() {
        normalize_identifier(node, &original);
    } else {
        normalize_identifier_path(node, &original, remaining);
    }
}

pub(crate) fn deserialize_required_query_with_identifiers<T, Q>(
    query: Q,
    missing_error: &str,
    context: &str,
    paths: &[&[&str]],
    array_fields: &[&str],
) -> Result<T, WasmSdkError>
where
    T: DeserializeOwned,
    Q: Into<JsValue>,
{
    let original = query.into();
    if original.is_null() || original.is_undefined() {
        return Err(WasmSdkError::invalid_argument(missing_error.to_string()));
    }

    // Preserve whole-object conversion and its errors before inspecting named fields.
    let mut value = platform_value_from_object(&original)
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid {context}: {e}")))?;
    for path in paths {
        normalize_identifier_path(&mut value, &original, path);
    }
    if let PlatformValue::Map(entries) = &mut value {
        for field in array_fields {
            let Some((_, PlatformValue::Array(elements))) = entries.iter_mut().find(
                |(key, _)| matches!(key, PlatformValue::Text(text) if text.as_str() == *field),
            ) else {
                continue;
            };
            let Ok(array) = Reflect::get(&original, &JsValue::from_str(field)) else {
                continue;
            };
            for (index, element) in elements.iter_mut().enumerate() {
                if let Ok(original) = Reflect::get(&array, &JsValue::from_f64(index as f64)) {
                    normalize_identifier(element, &original);
                }
            }
        }
    }
    platform_value::from_value(value)
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid {context}: fromObject: {e}")))
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
