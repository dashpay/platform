use dpp::dashcore::anyhow::Context;
use js_sys::Function;
use serde_json::Value;
use std::convert::{TryFrom, TryInto};
use wasm_bindgen::prelude::*;

pub trait ToSerdeJSONExt {
    fn to_serde_json_value(&self) -> Result<Value, JsValue>;
}

impl ToSerdeJSONExt for JsValue {
    fn to_serde_json_value(&self) -> Result<Value, JsValue> {
        to_serde_json_value(self)
    }
}

pub fn to_vec_js<T>(iter: impl IntoIterator<Item = T>) -> Vec<JsValue>
where
    T: Into<JsValue>,
{
    iter.into_iter().map(|v| v.into()).collect()
}

pub fn to_vec_of_serde_values(
    values: impl IntoIterator<Item = impl AsRef<JsValue>>,
) -> Result<Vec<Value>, JsValue> {
    values
        .into_iter()
        .map(|v| v.as_ref().to_serde_json_value())
        .collect()
}

pub fn into_vec_of<T>(iter: &[JsValue]) -> Vec<T>
where
    T: for<'de> serde::de::Deserialize<'de>,
{
    iter.iter()
        .map(|v| v.into_serde().expect("data malformed"))
        .collect()
}

pub fn into_vec<T: TryFrom<JsValue>>(
    values: Vec<JsValue>,
) -> Result<Vec<T>, <JsValue as TryInto<T>>::Error> {
    values
        .into_iter()
        .map(JsValue::try_into)
        .collect::<Result<Vec<T>, <JsValue as TryInto<T>>::Error>>()
}

pub fn to_serde_json_value(data: &JsValue) -> Result<Value, JsValue> {
    let data = stringify(data)?;
    let value: Value = serde_json::from_str(&data)
        .with_context(|| format!("cant convert {:#?} to serde json value", data))
        .map_err(|e| format!("{:#}", e))?;
    Ok(value)
}

pub fn stringify(data: &JsValue) -> Result<String, JsValue> {
    let replacer_func = Function::new_with_args(
        "key, value",
        "return value.type=='Buffer' ? value.data : value ",
    );

    let data_string: String =
        js_sys::JSON::stringify_with_replacer(data, &JsValue::from(replacer_func))?.into();

    Ok(data_string)
}
