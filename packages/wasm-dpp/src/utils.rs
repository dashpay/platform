use std::collections::BTreeMap;
use std::convert::TryInto;

use anyhow::{anyhow, bail, Context};
use dpp::{consensus::ConsensusError, ProtocolError};

use dpp::platform_value::Value;
use dpp::serialization::PlatformDeserializable;
use js_sys::{Function, Uint8Array};
use serde::de::DeserializeOwned;
use serde_json::Value as JsonValue;
use wasm_bindgen::{convert::RefFromWasmAbi, prelude::*};

use crate::{
    buffer::Buffer,
    errors::{from_dpp_err, RustConversionError},
};

pub trait ToSerdeJSONExt {
    fn with_serde_to_json_value(&self) -> Result<JsonValue, JsValue>;
    fn with_serde_to_platform_value(&self) -> Result<Value, JsValue>;
    /// Converts the `JsValue` into `platform::Value`. It's an expensive conversion,
    /// as `JsValue` must be stringified first
    fn with_serde_to_platform_value_map(&self) -> Result<BTreeMap<String, Value>, JsValue>;
    fn with_serde_into<D: DeserializeOwned>(&self) -> Result<D, JsValue>
    where
        D: for<'de> serde::de::Deserialize<'de> + 'static;
}

impl ToSerdeJSONExt for JsValue {
    /// Converts the `JsValue` into `serde_json::Value`. It's an expensive conversion,
    /// as `JsValue` must be stringified first
    fn with_serde_to_json_value(&self) -> Result<JsonValue, JsValue> {
        with_serde_to_json_value(self)
    }

    /// Converts the `JsValue` into `platform::Value`. It's an expensive conversion,
    /// as `JsValue` must be stringified first
    fn with_serde_to_platform_value(&self) -> Result<Value, JsValue> {
        with_serde_to_platform_value(self)
    }

    /// Converts the `JsValue` into `platform::Value`. It's an expensive conversion,
    /// as `JsValue` must be stringified first
    fn with_serde_to_platform_value_map(&self) -> Result<BTreeMap<String, Value>, JsValue> {
        self.with_serde_to_platform_value()?
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)
            .with_js_error()
    }

    /// converts the `JsValue` into any type that is supported by serde. It's an expensive conversion
    /// as the `jsValue` must be stringified first
    fn with_serde_into<D>(&self) -> Result<D, JsValue>
    where
        D: for<'de> serde::de::Deserialize<'de> + 'static,
    {
        with_serde_into(self)
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
) -> Result<Vec<JsonValue>, JsValue> {
    values
        .into_iter()
        .map(|v| v.as_ref().with_serde_to_json_value())
        .collect()
}

pub fn to_vec_of_platform_values(
    values: impl IntoIterator<Item = impl AsRef<JsValue>>,
) -> Result<Vec<Value>, JsValue> {
    values
        .into_iter()
        .map(|v| v.as_ref().with_serde_to_platform_value())
        .collect()
}

pub fn into_vec_of<T>(iter: &[JsValue]) -> Vec<T>
where
    T: for<'de> serde::de::Deserialize<'de>,
{
    iter.iter()
        .map(|v| serde_wasm_bindgen::from_value(v.clone()).expect("data malformed"))
        .collect()
}

pub fn with_serde_to_json_value(data: &JsValue) -> Result<JsonValue, JsValue> {
    let data = stringify(data)?;
    let value: JsonValue = serde_json::from_str(&data)
        .with_context(|| format!("cant convert {data:#?} to serde json value"))
        .map_err(|e| format!("{e:#}"))?;
    Ok(value)
}

pub fn with_serde_to_platform_value(data: &JsValue) -> Result<Value, JsValue> {
    Ok(with_serde_to_json_value(data)?.into())
}

pub fn with_serde_into<D>(data: &JsValue) -> Result<D, JsValue>
where
    D: for<'de> serde::de::Deserialize<'de> + 'static,
{
    let data = stringify(data)?;
    let value: D = serde_json::from_str(&data)
        .with_context(|| format!("cant convert {:#?} to serde json value", data))
        .map_err(|e| format!("{:#}", e))?;
    Ok(value)
}

pub fn stringify(data: &JsValue) -> Result<String, JsValue> {
    let replacer_func = Function::new_with_args(
        "key, value",
        "return (value != undefined && value.type=='Buffer')  ? value.data : value ",
    );

    let data_string: String =
        js_sys::JSON::stringify_with_replacer(data, &JsValue::from(replacer_func))?.into();

    Ok(data_string)
}

pub trait WithJsError<T> {
    /// Converts the error into JsValue
    fn with_js_error(self) -> Result<T, JsValue>;
}

impl<T> WithJsError<T> for Result<T, anyhow::Error> {
    fn with_js_error(self) -> Result<T, JsValue> {
        match self {
            Ok(ok) => Ok(ok),
            Err(error) => Err(format!("{error:#}").into()),
        }
    }
}

impl<T> WithJsError<T> for Result<T, ProtocolError> {
    fn with_js_error(self) -> Result<T, JsValue> {
        match self {
            Ok(ok) => Ok(ok),
            Err(error) => Err(from_dpp_err(error)),
        }
    }
}

impl<T> WithJsError<T> for Result<T, serde_json::Error> {
    fn with_js_error(self) -> Result<T, JsValue> {
        match self {
            Ok(ok) => Ok(ok),
            Err(error) => Err(RustConversionError::from(error).to_js_value()),
        }
    }
}

pub trait IntoWasm {
    fn to_wasm<T: RefFromWasmAbi<Abi = u32>>(&self, class_name: &str)
        -> Result<T::Anchor, JsValue>;
}

impl IntoWasm for JsValue {
    fn to_wasm<T: RefFromWasmAbi<Abi = u32>>(
        &self,
        class_name: &str,
    ) -> Result<T::Anchor, JsValue> {
        generic_of_js_val::<T>(self, class_name)
    }
}

pub fn generic_of_js_val<T: RefFromWasmAbi<Abi = u32>>(
    js_value: &JsValue,
    class_name: &str,
) -> Result<T::Anchor, JsValue> {
    if !js_value.is_object() {
        return Err(JsError::new(
            format!("Value supplied as {} is not an object", class_name).as_str(),
        )
        .into());
    }

    let ctor_name = js_sys::Object::get_prototype_of(js_value)
        .constructor()
        .name();

    if ctor_name == class_name {
        let ptr = js_sys::Reflect::get(js_value, &JsValue::from_str("__wbg_ptr"))?;
        let ptr_u32: u32 = ptr
            .as_f64()
            .ok_or_else(|| JsValue::from(JsError::new("Invalid JS object pointer")))?
            as u32;
        let reference = unsafe { T::ref_from_abi(ptr_u32) };
        Ok(reference)
    } else {
        let error_string = format!(
            "JS object constructor name mismatch. Expected {}, provided {}.",
            class_name, ctor_name
        );
        Err(JsError::new(&error_string).into())
    }
}

pub const SKIP_VALIDATION_PROPERTY_NAME: &str = "skipValidation";

pub fn get_bool_from_options(
    options: JsValue,
    property: &str,
    default: bool,
) -> Result<bool, JsValue> {
    if options.is_object() {
        let val2 = options.with_serde_to_json_value()?;
        let kek = val2
            .as_object()
            .ok_or_else(|| JsError::new("Can't parse options"))?;
        let kek2 = kek
            .get(property)
            .ok_or_else(|| JsError::new(&format!("Can't get property {} of options", property)))?;
        Ok(kek2
            .as_bool()
            .ok_or_else(|| JsError::new(&format!("Option {} is not a boolean", property)))?)
    } else {
        Ok(default)
    }
}

#[allow(dead_code)]
#[deprecated(note = "This function is marked as unused.")]
#[allow(deprecated)]
pub fn get_class_name(value: &JsValue) -> String {
    js_sys::Object::get_prototype_of(value)
        .constructor()
        .name()
        .into()
}

#[allow(dead_code)]
#[deprecated(note = "This function is marked as unused.")]
#[allow(deprecated)]
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

// The trait `Inner` provides better flexibility and visibility when you need to switch
// between WASM structure and original structure.
pub(crate) trait Inner {
    type InnerItem;

    fn into_inner(self) -> Self::InnerItem;
    fn inner(&self) -> &Self::InnerItem;
    fn inner_mut(&mut self) -> &mut Self::InnerItem;
}

pub(crate) fn consensus_errors_from_buffers(
    errors: Vec<Buffer>,
) -> Result<Vec<ConsensusError>, JsValue> {
    errors
        .iter()
        .map(|error_buffer| {
            Uint8Array::new_with_byte_offset_and_length(
                &error_buffer.buffer(),
                error_buffer.byte_offset(),
                error_buffer.length(),
            )
            .to_vec()
        })
        .map(|error_bytes| {
            ConsensusError::deserialize_from_bytes(&error_bytes.to_vec()).with_js_error()
        })
        .collect::<Result<Vec<ConsensusError>, JsValue>>()
}
