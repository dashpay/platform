use std::convert::{TryFrom, TryInto};
use wasm_bindgen::prelude::*;

pub fn to_vec_js<T>(iter: impl IntoIterator<Item = T>) -> Vec<JsValue>
where
    T: Into<JsValue>,
{
    iter.into_iter().map(|v| v.into()).collect()
}

pub fn from_vec_js<T>(iter: &[JsValue]) -> Vec<T>
where
    T: for<'de> serde::de::Deserialize<'de>,
{
    iter.iter()
        .map(|v| v.into_serde().expect("data malformed"))
        .collect()
}

pub fn into_vec<T: TryFrom<JsValue>>(values: Vec<JsValue>) -> Result<Vec<T>, <JsValue as TryInto<T>>::Error> {
    values.into_iter().map(JsValue::try_into).collect::<Result<
        Vec<T>,
        <JsValue as TryInto<T>>::Error,
    >>()
}
