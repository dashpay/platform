use dpp::prelude::Identifier;
use itertools::Itertools;
use serde_json::Value;
use wasm_bindgen::prelude::*;

use crate::bail_js;
use crate::buffer::Buffer;
use crate::utils::ToSerdeJSONExt;
use crate::utils::WithJsError;
use dpp::platform_value::string_encoding::Encoding;
use dpp::ProtocolError;

#[wasm_bindgen(raw_module = "../identifier/Identifier.js")]
extern "C" {
    #[derive(Debug, Clone)]
    #[wasm_bindgen(js_name = default)]
    pub type IdentifierWrapper; // Rename to IdentifierJS?

    #[wasm_bindgen(constructor, js_class=default)]
    pub fn new(buffer: JsValue) -> IdentifierWrapper;

    #[wasm_bindgen(method, js_name=toBuffer)]
    pub fn to_buffer(this: &IdentifierWrapper) -> Vec<u8>;
}

impl From<Identifier> for IdentifierWrapper {
    fn from(s: Identifier) -> Self {
        IdentifierWrapper::new(Buffer::from_bytes(s.as_slice()).into())
    }
}

impl From<IdentifierWrapper> for Identifier {
    fn from(s: IdentifierWrapper) -> Self {
        Identifier::from_bytes(&s.to_buffer()).unwrap()
    }
}

impl From<&IdentifierWrapper> for Identifier {
    fn from(s: &IdentifierWrapper) -> Self {
        Identifier::from_bytes(&s.to_buffer()).unwrap()
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
