

pub use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::errors::from_dpp_err;
use dpp::identifier;
use js_sys::Uint8Array;

#[derive(Serialize, Deserialize, PartialEq, Eq)]
enum IdentifierSource {
    String(String),
    Buffer(Vec<u8>),
}

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);

    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn log(a: &str);
}

#[wasm_bindgen(js_name = Identifier, inspectable)]
pub struct IdentifierWrapper {
    wrapped: identifier::Identifier,
}

impl std::convert::From<identifier::Identifier> for IdentifierWrapper {
    fn from(s: identifier::Identifier) -> Self {
        IdentifierWrapper { wrapped: s }
    }
}

#[wasm_bindgen(js_class = Identifier)]
impl IdentifierWrapper {
    #[wasm_bindgen(constructor)]
    pub fn new(buffer: Vec<u8>) -> Result<IdentifierWrapper, JsValue> {
        // TODO: remove unwrap
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

    #[wasm_bindgen(js_name = fromString)]
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

    #[wasm_bindgen(js_name = toBuffer)]
    pub fn to_buffer(&self) -> Uint8Array {
        js_sys::Uint8Array::from(self.wrapped.buffer.as_slice())
    }

    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> String {
        self.to_string(None)
    }

    #[wasm_bindgen(js_name = toString)]
    pub fn to_string(&self, encoding: Option<String>) -> String {
        // Converting string to a string slice. Rust interfaces work
        // with immutable string slices more often, while js interop accepts mutable String.
        // as_deref dereferences value in the Option
        // dereferencing is accessing the underlying value of the reference, which in
        // case of the string would be a string slice
        self.wrapped
            .to_string_with_encoding_string(encoding.as_deref())
    }

    // #[wasm_bindgen(js_name = encodeCBOR)]
    // pub fn encode_cbor(&self) {
    //     self.wrapped.buffer
    // }

    #[wasm_bindgen(getter)]
    pub fn length(&self) -> usize {
        self.wrapped.buffer.len()
    }

    pub fn inner(&self) -> Vec<u8> {
        self.wrapped.buffer.to_vec()
    }

    // #[wasm_bindgen(js_name = valueOf)]
    // pub fn value_of(&self) -> js_sys::Uint8Array {
    //     // let js_arr: js_sys::Array = self.inner().into_iter().map(JsValue::from).collect();
    //     js_sys::Uint8Array::from(self.wrapped.buffer.as_slice())
    // }
}
