use dpp::prelude::Identifier;
use dpp::util::string_encoding::Encoding;
use itertools::Itertools;
pub use serde::{Deserialize, Serialize};
use serde_json::Value;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::bail_js;
use crate::buffer::Buffer;
use crate::errors::from_dpp_err;
use crate::utils::ToSerdeJSONExt;
use crate::utils::WithJsError;
use dpp::identifier;

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

#[wasm_bindgen(js_name=Identifier, inspectable)]
pub struct IdentifierWrapper {
    wrapped: identifier::Identifier,
}

impl std::convert::From<identifier::Identifier> for IdentifierWrapper {
    fn from(s: identifier::Identifier) -> Self {
        IdentifierWrapper { wrapped: s }
    }
}

impl std::convert::From<&IdentifierWrapper> for identifier::Identifier {
    fn from(s: &IdentifierWrapper) -> Self {
        s.wrapped.clone()
    }
}

#[wasm_bindgen(js_class=Identifier)]
impl IdentifierWrapper {
    #[wasm_bindgen(constructor)]
    pub fn new(buffer: Vec<u8>) -> Result<IdentifierWrapper, JsValue> {
        let identifier = identifier::Identifier::from_bytes(&buffer).map_err(from_dpp_err)?;

        Ok(IdentifierWrapper {
            wrapped: identifier,
        })
    }

    pub fn from(value: JsValue, encoding: Option<String>) -> Result<IdentifierWrapper, JsValue> {
        if value.is_string() {
            let string = value.as_string().unwrap();
            Ok(IdentifierWrapper::from_string(string, encoding))
        } else if value.has_type::<js_sys::Uint8Array>() {
            let vec = value.dyn_into::<js_sys::Uint8Array>()?.to_vec();
            IdentifierWrapper::new(vec)
        } else {
            Err(JsValue::from(
                "Identifier.from received an unexpected value",
            ))
        }
    }

    #[wasm_bindgen(js_name=fromString)]
    pub fn from_string(value: String, encoding: Option<String>) -> IdentifierWrapper {
        // TODO: remove unwrap
        let identifier = identifier::Identifier::from_string_with_encoding_string(
            &value[..],
            encoding.as_deref(),
        )
        .unwrap();

        IdentifierWrapper {
            wrapped: identifier,
        }
    }

    #[wasm_bindgen(js_name=toBuffer)]
    pub fn to_buffer(&self) -> Buffer {
        Buffer::from_bytes(&self.wrapped.buffer)
    }

    #[wasm_bindgen(js_name=toJSON)]
    pub fn to_json(&self) -> String {
        self.to_string(None)
    }

    #[wasm_bindgen(js_name=toString)]
    pub fn to_string(&self, encoding: Option<String>) -> String {
        // Converting string to a string slice. Rust interfaces work
        // with immutable string slices more often, while js interop accepts mutable String.
        // as_deref dereferences value in the Option
        // dereferencing is accessing the underlying value of the reference, which in
        // case of the string would be a string slice
        self.wrapped
            .to_string_with_encoding_string(encoding.as_deref())
    }

    #[wasm_bindgen(getter)]
    pub fn length(&self) -> usize {
        self.wrapped.buffer.len()
    }

    #[wasm_bindgen(js_name=toBytes)]
    pub fn to_bytes(&self) -> Vec<u8> {
        self.wrapped.buffer.to_vec()
    }

    #[wasm_bindgen(js_name=clone)]
    pub fn deep_clone(&self) -> IdentifierWrapper {
        IdentifierWrapper {
            wrapped: self.wrapped.clone(),
        }
    }
}

impl IdentifierWrapper {
    pub fn inner(self) -> Identifier {
        self.wrapped
    }
}

/// tries to create identifier from
pub fn identifier_from_js_value(js_value: &JsValue) -> Result<Identifier, JsValue> {
    let value = js_value.with_serde_to_json_value()?;
    match value {
        Value::Array(arr) => {
            let bytes: Vec<u8> = arr.into_iter().map(value_to_u8).try_collect()?;
            Identifier::from_bytes(&bytes).with_js_error()
        }
        Value::String(string) => Identifier::from_string(&string, Encoding::Base58).with_js_error(),
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
