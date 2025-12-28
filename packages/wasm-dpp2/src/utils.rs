use crate::error::{WasmDppError, WasmDppResult};
use anyhow::{anyhow, bail};
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
use dpp::util::hash::hash_double_to_vec;
use js_sys::Error as JsError;
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;
use std::convert::TryInto;
use wasm_bindgen::convert::RefFromWasmAbi;
use wasm_bindgen::{JsCast, JsValue};

/// Extension trait for extracting error messages from JsValue
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

/// Extension trait for converting JsValue to serde/platform values
pub trait ToSerdeJSONExt {
    fn with_serde_to_json_value(&self) -> WasmDppResult<JsonValue>;
    fn with_serde_to_platform_value(&self) -> WasmDppResult<Value>;
    fn with_serde_to_platform_value_map(&self) -> WasmDppResult<BTreeMap<String, Value>>;
}

impl ToSerdeJSONExt for JsValue {
    fn with_serde_to_json_value(&self) -> WasmDppResult<JsonValue> {
        crate::serialization::js_value_to_json(self)
    }

    fn with_serde_to_platform_value(&self) -> WasmDppResult<Value> {
        Ok(self.with_serde_to_json_value()?.into())
    }

    fn with_serde_to_platform_value_map(&self) -> WasmDppResult<BTreeMap<String, Value>> {
        self.with_serde_to_platform_value()?
            .into_btree_string_map()
            .map_err(|err| WasmDppError::invalid_argument(err.to_string()))
    }
}

/// Trait for converting JsValue to a WASM type by reading its internal pointer
pub trait IntoWasm {
    fn to_wasm<T: RefFromWasmAbi<Abi = u32>>(&self, class_name: &str) -> WasmDppResult<T::Anchor>;
}

impl IntoWasm for JsValue {
    fn to_wasm<T: RefFromWasmAbi<Abi = u32>>(&self, class_name: &str) -> WasmDppResult<T::Anchor> {
        generic_of_js_val::<T>(self, class_name)
    }
}

/// Convert a JsValue to a WASM type by reading its internal pointer
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

/// Get the `__type` property from a JsValue (used for WASM class identification)
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

/// Convert a JS Number or BigInt to u64
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

/// Convert a JS Number to u64 with validation
fn convert_number_to_u64(js_number: js_sys::Number) -> Result<u64, anyhow::Error> {
    if let Some(float_number) = js_number.as_f64() {
        if float_number.is_nan() || float_number.is_infinite() {
            bail!("received an invalid number: the number is either NaN or Inf")
        }
        if float_number < 0. {
            bail!("received an invalid number: the number is negative");
        }
        if float_number.fract() != 0. {
            bail!("received an invalid number: the number is fractional")
        }
        if float_number > u64::MAX as f64 {
            bail!("received an invalid number: the number is > u64::max")
        }

        return Ok(float_number as u64);
    }
    bail!("the value is not a number")
}

/// Generate a document ID using the v0 algorithm
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
