use crate::errors::from_js_error;
use dpp::util::entropy_generator::EntropyGenerator;
use std::convert::TryInto;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[wasm_bindgen]
extern "C" {
    #[derive(Clone)]
    pub type ExternalEntropyGenerator;

    #[wasm_bindgen(catch, structural, method)]
    pub fn generate(this: &ExternalEntropyGenerator) -> Result<JsValue, JsValue>;
}

impl EntropyGenerator for ExternalEntropyGenerator {
    fn generate(&self) -> anyhow::Result<[u8; 32]> {
        let js_value = ExternalEntropyGenerator::generate(self).map_err(from_js_error)?;

        if !js_value.has_type::<js_sys::Uint8Array>() {
            anyhow::bail!("Entropy generator should return Buffer");
        }

        let vec = js_value
            .dyn_into::<js_sys::Uint8Array>()
            .map_err(from_js_error)?
            .to_vec();

        let bytes = vec.try_into().map_err(|_| {
            anyhow::anyhow!("Bad entropy generator provided: should return 32 bytes")
        })?;

        Ok(bytes)
    }
}
