use crate::bls_adapter::{BlsAdapter, JsBlsAdapter};

use crate::utils::{to_vec_of_serde_values, ToSerdeJSONExt};
use crate::validation_result::ValidationResultWasm;
use dpp::identity::validation::{PublicKeysValidator, TPublicKeysValidator};

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = PublicKeysValidator)]
pub struct PublicKeysValidatorWasm(PublicKeysValidator<BlsAdapter>);

#[wasm_bindgen(js_class = PublicKeysValidator)]
impl PublicKeysValidatorWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(adapter: JsBlsAdapter) -> PublicKeysValidatorWasm {
        Self(PublicKeysValidator::new(BlsAdapter(adapter)).unwrap())
    }

    #[wasm_bindgen(js_name=validateKeys)]
    pub fn validate_keys(
        &self,
        public_keys: js_sys::Array,
    ) -> Result<ValidationResultWasm, JsValue> {
        let raw_public_keys = to_vec_of_serde_values(public_keys.iter())?;

        self.0
            .validate_keys(&raw_public_keys)
            .map(ValidationResultWasm::from)
            .map_err(|e| JsValue::from(e.to_string()))
    }

    #[wasm_bindgen(js_name=validatePublicKeyStructure)]
    pub fn validate_public_key_structure(
        &self,
        public_key: JsValue,
    ) -> Result<ValidationResultWasm, JsValue> {
        let pk_serde_json = public_key.with_serde_to_json_value()?;

        self.0
            .validate_public_key_structure(&pk_serde_json)
            .map(ValidationResultWasm::from)
            .map_err(|e| JsValue::from(e.to_string()))
    }
}
