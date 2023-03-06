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

impl ProtocolVersionValidatorWasm {
    pub fn protocol_version(&self) -> u32 {
        self.0.protocol_version()
    }
}

impl From<ProtocolVersionValidator> for ProtocolVersionValidatorWasm {
    fn from(doc_validator: ProtocolVersionValidator) -> Self {
        ProtocolVersionValidatorWasm(doc_validator)
    }
}

impl From<&ProtocolVersionValidator> for ProtocolVersionValidatorWasm {
    fn from(doc_validator: &ProtocolVersionValidator) -> Self {
        ProtocolVersionValidatorWasm(doc_validator.clone())
    }
}

impl From<ProtocolVersionValidatorWasm> for ProtocolVersionValidator {
    fn from(val: ProtocolVersionValidatorWasm) -> Self {
        val.0
    }
}

impl From<&ProtocolVersionValidatorWasm> for ProtocolVersionValidator {
    fn from(val: &ProtocolVersionValidatorWasm) -> Self {
        val.0.clone()
    }
}
