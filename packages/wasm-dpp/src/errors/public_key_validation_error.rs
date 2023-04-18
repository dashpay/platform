use dpp::PublicKeyValidationError;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name=PublicKeyValidationError)]
#[derive(Clone, Debug)]
pub struct PublicKeyValidationErrorWasm {
    inner: PublicKeyValidationError,
}

impl From<PublicKeyValidationError> for PublicKeyValidationErrorWasm {
    fn from(e: PublicKeyValidationError) -> Self {
        Self { inner: e }
    }
}

#[wasm_bindgen(js_class=PublicKeyValidationError)]
impl PublicKeyValidationErrorWasm {
    #[wasm_bindgen(getter)]
    pub fn message(&self) -> JsValue {
        self.inner.message().into()
    }
}
