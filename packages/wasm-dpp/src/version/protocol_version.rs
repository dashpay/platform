use dpp::version::ProtocolVersionValidator;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=ProtocolVersionValidator)]
pub struct ProtocolVersionValidatorWasm(ProtocolVersionValidator);

#[wasm_bindgen(js_class=ProtocolVersionValidator)]
impl ProtocolVersionValidatorWasm {
    // TODO should the constructor be without parameters?
    #[wasm_bindgen(constructor)]
    pub fn new() -> ProtocolVersionValidatorWasm {
        ProtocolVersionValidatorWasm(ProtocolVersionValidator::default())
    }
}

impl From<ProtocolVersionValidator> for ProtocolVersionValidatorWasm {
    fn from(doc_validator: ProtocolVersionValidator) -> Self {
        ProtocolVersionValidatorWasm(doc_validator)
    }
}

impl Into<ProtocolVersionValidator> for ProtocolVersionValidatorWasm {
    fn into(self) -> ProtocolVersionValidator {
        self.0
    }
}
