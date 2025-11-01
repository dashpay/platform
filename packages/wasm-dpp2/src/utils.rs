use crate::error::{WasmDppError, WasmDppResult};
use anyhow::{Context, anyhow, bail};
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::platform_value::string_encoding::Encoding::Base58;
use dpp::util::hash::hash_double_to_vec;
use js_sys::{Error as JsError, Function, Uint8Array};
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use std::convert::TryInto;
use wasm_bindgen::convert::RefFromWasmAbi;
use wasm_bindgen::{JsCast, JsValue};

pub fn stringify_wasm(data: &JsValue) -> WasmDppResult<String> {
    let replacer_func = Function::new_with_args(
        "key, value",
        "return (value != undefined && value.type=='Buffer')  ? value.data : value ",
    );

    let data_string = js_sys::JSON::stringify_with_replacer(data, &JsValue::from(replacer_func))
        .map_err(|_| WasmDppError::serialization("Failed to stringify value"))?;

    Ok(data_string.into())
}

pub trait JsValueExt {
    fn error_message(&self) -> String;
}

impl JsValueExt for JsValue {
    fn error_message(&self) -> String {
        if self.is_null() || self.is_undefined() {
            return "JavaScript error: value is null or undefined".to_string();
        }

        if let Some(js_error) = self.dyn_ref::<JsError>() {
            return js_error.message().into();
        }

        if let Some(message) = self.as_string() {
            return message;
        }

        js_sys::JSON::stringify(self)
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| "Unknown JavaScript error".to_string())
    }
}

pub fn with_serde_to_json_value_wasm(data: JsValue) -> WasmDppResult<JsonValue> {
    let data = stringify_wasm(&data)?;
    serde_json::from_str(&data).map_err(|e| {
        WasmDppError::serialization(format!(
            "unable to convert value to serde_json::Value: {e:#}"
        ))
    })
}

pub fn with_serde_to_platform_value_wasm(data: &JsValue) -> WasmDppResult<Value> {
    Ok(with_serde_to_json_value_wasm(data.clone())?.into())
}

pub trait ToSerdeJSONExt {
    fn with_serde_to_json_value(&self) -> WasmDppResult<JsonValue>;
    fn with_serde_to_platform_value(&self) -> WasmDppResult<Value>;
    /// Converts the `JsValue` into `platform::Value`. It's an expensive conversion,
    /// as `JsValue` must be stringified first
    fn with_serde_to_platform_value_map(&self) -> WasmDppResult<BTreeMap<String, Value>>;
}

impl ToSerdeJSONExt for JsValue {
    /// Converts the `JsValue` into `serde_json::Value`. It's an expensive conversion,
    /// as `JsValue` must be stringified first
    fn with_serde_to_json_value(&self) -> WasmDppResult<JsonValue> {
        with_serde_to_json_value(self.clone())
    }

    /// Converts the `JsValue` into `platform::Value`. It's an expensive conversion,
    /// as `JsValue` must be stringified first
    fn with_serde_to_platform_value(&self) -> WasmDppResult<Value> {
        with_serde_to_platform_value(self)
    }

    /// Converts the `JsValue` into `platform::Value`. It's an expensive conversion,
    /// as `JsValue` must be stringified first
    fn with_serde_to_platform_value_map(&self) -> WasmDppResult<BTreeMap<String, Value>> {
        self.with_serde_to_platform_value()?
            .into_btree_string_map()
            .map_err(|err| WasmDppError::invalid_argument(err.to_string()))
    }
}

pub fn to_vec_js<T>(iter: impl IntoIterator<Item = T>) -> Vec<JsValue>
where
    T: Into<JsValue>,
{
    iter.into_iter().map(|v| v.into()).collect()
}

#[allow(dead_code)]
#[deprecated(note = "This function is marked as unused.")]
#[allow(deprecated)]
pub fn to_vec_of_serde_values(
    values: impl IntoIterator<Item = impl AsRef<JsValue>>,
) -> WasmDppResult<Vec<JsonValue>> {
    values
        .into_iter()
        .map(|v| v.as_ref().with_serde_to_json_value())
        .collect()
}

pub fn to_vec_of_platform_values(
    values: impl IntoIterator<Item = impl AsRef<JsValue>>,
) -> WasmDppResult<Vec<Value>> {
    values
        .into_iter()
        .map(|v| v.as_ref().with_serde_to_platform_value())
        .collect()
}

pub fn with_serde_to_json_value(data: JsValue) -> WasmDppResult<JsonValue> {
    let data = stringify(&data)?;
    let value: JsonValue = serde_json::from_str(&data)
        .with_context(|| format!("cant convert {data:#?} to serde json value"))
        .map_err(|e| WasmDppError::serialization(format!("{e:#}")))?;
    Ok(value)
}

pub fn with_serde_to_platform_value(data: &JsValue) -> WasmDppResult<Value> {
    Ok(with_serde_to_json_value(data.clone())?.into())
}

pub fn stringify(data: &JsValue) -> WasmDppResult<String> {
    let replacer_func = Function::new_with_args(
        "key, value",
        "return (value != undefined && value.type=='Buffer')  ? value.data : value ",
    );

    let data_string: String =
        js_sys::JSON::stringify_with_replacer(data, &JsValue::from(replacer_func))
            .map_err(|err| {
                let message = err.error_message();
                WasmDppError::serialization(format!(
                    "unable to stringify value to JSON: {}",
                    message
                ))
            })?
            .into();

    Ok(data_string)
}

pub trait IntoWasm {
    fn to_wasm<T: RefFromWasmAbi<Abi = u32>>(&self, class_name: &str) -> WasmDppResult<T::Anchor>;
}

impl IntoWasm for JsValue {
    fn to_wasm<T: RefFromWasmAbi<Abi = u32>>(&self, class_name: &str) -> WasmDppResult<T::Anchor> {
        generic_of_js_val::<T>(self, class_name)
    }
}

pub fn generic_of_js_val<T: RefFromWasmAbi<Abi = u32>>(
    js_value: &JsValue,
    class_name: &str,
) -> WasmDppResult<T::Anchor> {
    if !js_value.is_object() {
        return Err(WasmDppError::invalid_argument(format!(
            "Value supplied as {} is not an object",
            class_name
        )));
    }

    let ctor_name = get_class_type(js_value)?;

    if ctor_name == class_name {
        let ptr =
            js_sys::Reflect::get(js_value, &JsValue::from_str("__wbg_ptr")).map_err(|err| {
                let message = err.error_message();
                WasmDppError::generic(format!(
                    "failed to read internal pointer from JS object '{}': {}",
                    class_name, message
                ))
            })?;
        let ptr_u32: u32 = ptr
            .as_f64()
            .ok_or_else(|| WasmDppError::invalid_argument("Invalid JS object pointer"))?
            as u32;
        let reference = unsafe { T::ref_from_abi(ptr_u32) };
        Ok(reference)
    } else {
        let error_string = format!(
            "JS object constructor name mismatch. Expected {}, provided {}.",
            class_name, ctor_name
        );
        Err(WasmDppError::invalid_argument(error_string))
    }
}

pub fn get_class_type(value: &JsValue) -> WasmDppResult<String> {
    let class_type = js_sys::Reflect::get(value, &JsValue::from_str("__type")).map_err(|err| {
        let message = err.error_message();
        WasmDppError::generic(format!(
            "failed to read '__type' property from JS value: {}",
            message
        ))
    })?;

    Ok(class_type.as_string().unwrap_or_default())
}

pub fn try_to_u64(value: JsValue) -> Result<u64, anyhow::Error> {
    if value.is_bigint() {
        js_sys::BigInt::new(&value)
            .map_err(|e| anyhow!("unable to create bigInt: {}", e.to_string()))?
            .try_into()
            .map_err(|e| anyhow!("conversion of BigInt to u64 failed: {:#}", e))
    } else if value.as_f64().is_some() {
        let number = js_sys::Number::from(value);
        convert_number_to_u64(number)
    } else {
        bail!("supported types are Number or BigInt")
    }
}

pub fn convert_number_to_u64(js_number: js_sys::Number) -> Result<u64, anyhow::Error> {
    if let Some(float_number) = js_number.as_f64() {
        if float_number.is_nan() || float_number.is_infinite() {
            bail!("received an invalid timestamp: the number is either NaN or Inf")
        }
        if float_number < 0. {
            bail!("received an invalid timestamp: the number is negative");
        }
        if float_number.fract() != 0. {
            bail!("received an invalid timestamp: the number is fractional")
        }
        if float_number > u64::MAX as f64 {
            bail!("received an invalid timestamp: the number is > u64::max")
        }

        return Ok(float_number as u64);
    }
    bail!("the value is not a number")
}

pub fn generate_document_id_v0(
    contract_id: &Identifier,
    owner_id: &Identifier,
    document_type_name: &str,
    entropy: &[u8],
) -> WasmDppResult<Identifier> {
    let mut buf: Vec<u8> = vec![];

    buf.extend_from_slice(&contract_id.to_buffer());
    buf.extend_from_slice(&owner_id.to_buffer());
    buf.extend_from_slice(document_type_name.as_bytes());
    buf.extend_from_slice(entropy);

    Identifier::from_bytes(&hash_double_to_vec(&buf))
        .map_err(|err| WasmDppError::invalid_argument(err.to_string()))
}

// TODO: Refactor, code bellow

// if let Ok(obj) = input.dyn_into::<MyClass>() {
// // received wasm object
// web_sys::console::log_1(&format!("MyClass: {}", obj.value()).into());
// } else if let Some(s) = input.as_string() {
// // received string
// web_sys::console::log_1(&format!("String: {}", s).into());
// } else {
// web_sys::console::error_1(&"Expected string | MyClass".into());
// }

// Try to extract Identifier from **stringified** identifier_utils.
// The `js_value` can be a stringified instance of: `Identifier`, `Buffer` or `Array`
pub fn identifier_from_js_value(js_value: &JsValue) -> WasmDppResult<Identifier> {
    if js_value.is_undefined() || js_value.is_null() {
        return Err(WasmDppError::invalid_argument(
            "the identifier cannot be null or undefined",
        ));
    }

    match js_value.is_string() {
        true => Identifier::from_string(js_value.as_string().unwrap_or("".into()).as_str(), Base58)
            .map_err(|err| WasmDppError::invalid_argument(err.to_string())),
        false => match js_value.is_object() || js_value.is_array() {
            true => {
                let uint8_array = Uint8Array::from(js_value.clone());
                let bytes = uint8_array.to_vec();

                Identifier::from_bytes(&bytes)
                    .map_err(|err| WasmDppError::invalid_argument(err.to_string()))
            }
            false => Err(WasmDppError::invalid_argument(
                "Invalid ID. Expected array or string",
            )),
        },
    }
}
