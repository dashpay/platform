use dpp::CompatibleProtocolVersionIsNotDefinedError;
use thiserror::Error;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=CompatibleProtocolVersionIsNotDefinedError)]
#[derive(Error, Debug)]
#[error(transparent)]
pub struct CompatibleProtocolVersionIsNotDefinedErrorWasm(
    CompatibleProtocolVersionIsNotDefinedError,
);

impl CompatibleProtocolVersionIsNotDefinedErrorWasm {
    pub fn new(err: CompatibleProtocolVersionIsNotDefinedError) -> Self {
        Self(err)
    }
}

#[wasm_bindgen(js_class=CompatibleProtocolVersionIsNotDefinedError)]
impl CompatibleProtocolVersionIsNotDefinedErrorWasm {
    #[wasm_bindgen(js_name=getCurrentProtocolVersion)]
    pub fn current_protocol_version(&self) -> u32 {
        self.0.current_protocol_version()
    }
}
