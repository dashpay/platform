use dpp::prelude::Identifier;
use itertools::Itertools;
pub use serde::{Deserialize, Serialize};
use serde_json::Value;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::bail_js;
use crate::buffer::Buffer;
use crate::utils::Inner;
use crate::utils::ToSerdeJSONExt;
use crate::utils::WithJsError;
use dpp::platform_value::string_encoding::Encoding;
use dpp::{identifier, ProtocolError};

#[derive(Serialize, Deserialize, PartialEq, Eq)]
enum IdentifierSource {
    String(String),
    Buffer(Vec<u8>),
}

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);

    #[wasm_bindgen(js_namespace = console, js_name=log)]
    fn log(a: &str);
}

#[wasm_bindgen(raw_module = "../lib/identifier/Identifier.js")]
extern "C" {
    #[derive(Debug, Clone)]
    pub type IdentifierWrapper;

    #[wasm_bindgen(structural, js_name=Identifier)]
    fn IdentifierJS(buffer: &JsValue) -> IdentifierWrapper;

    #[wasm_bindgen(structural, js_name=IdentifierError)]
    fn IdentifierError() -> JsValue;

    #[wasm_bindgen(structural, js_name=test)]
    fn test() -> JsValue;

    // TODO: return Vec<u8>?
    #[wasm_bindgen(structural, method, js_name=toBuffer)]
    pub fn to_buffer(this: &IdentifierWrapper) -> JsValue;
}

impl From<identifier::Identifier> for IdentifierWrapper {
    fn from(s: identifier::Identifier) -> Self {
        IdentifierWasm(Buffer::from_bytes(s.as_slice()).into())
    }
}

impl From<[u8; 32]> for IdentifierWrapper {
    fn from(s: [u8; 32]) -> Self {
        IdentifierWasm(Buffer::from_bytes(&s).into())
    }
}

impl From<&IdentifierWrapper> for Identifier {
    fn from(s: &IdentifierWrapper) -> Self {
        let buffer = s.to_buffer();
        // TODO: do without dyn_into?
        let vec = buffer.dyn_into::<js_sys::Uint8Array>().unwrap().to_vec();
        Identifier::from_bytes(&vec).unwrap()
    }
}

impl From<&mut IdentifierWrapper> for Identifier {
    fn from(s: &mut IdentifierWrapper) -> Self {
        let buffer = s.to_buffer();
        // TODO: do without dyn_into?
        let vec = buffer.dyn_into::<js_sys::Uint8Array>().unwrap().to_vec();
        Identifier::from_bytes(&vec).unwrap()
    }
}

impl From<IdentifierWrapper> for Identifier {
    fn from(s: IdentifierWrapper) -> Self {
        let buffer = s.to_buffer();
        // TODO: do without dyn_into?
        let vec = buffer.dyn_into::<js_sys::Uint8Array>().unwrap().to_vec();
        Identifier::from_bytes(&vec).unwrap()
    }
}

// TODO: remove
pub fn IdentifierWasm(js_value: JsValue) -> IdentifierWrapper {
    IdentifierJS(&js_value)
}

// TODO: remove
impl IdentifierWrapper {
    pub fn new(js_value: JsValue) -> Result<IdentifierWrapper, JsValue> {
        Ok(IdentifierWasm(js_value))
    }
}

// Try to extract Identifier from **stringified** identifier.
// The `js_value` can be a stringified instance of: `Identifier`, `Buffer` or `Array`
pub(crate) fn identifier_from_js_value(js_value: &JsValue) -> Result<Identifier, JsValue> {
    if js_value.is_undefined() || js_value.is_null() {
        bail_js!("the identifier cannot be null or undefined")
    }
    let value = js_value.with_serde_to_json_value()?;
    match value {
        Value::Array(arr) => {
            let bytes: Vec<u8> = arr.into_iter().map(value_to_u8).try_collect()?;
            Identifier::from_bytes(&bytes)
                .map_err(ProtocolError::ValueError)
                .with_js_error()
        }
        Value::String(string) => Identifier::from_string(&string, Encoding::Base58)
            .map_err(ProtocolError::ValueError)
            .with_js_error(),
        _ => {
            bail_js!("Invalid ID. Expected array or string")
        }
    }
}

fn value_to_u8(v: Value) -> Result<u8, JsValue> {
    let number = v
        .as_u64()
        .ok_or_else(|| format!("failed converting {} into u64", v))?;
    if number > u8::MAX as u64 {
        bail_js!("the integer in the array isn't a byte: {}", number);
    }
    Ok(number as u8)
}
