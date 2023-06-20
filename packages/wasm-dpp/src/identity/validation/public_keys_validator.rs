use crate::bls_adapter::{BlsAdapter, JsBlsAdapter};

use crate::utils::{to_vec_of_platform_values, ToSerdeJSONExt};
use crate::validation::ValidationResultWasm;
use dpp::identity::validation::{
    PublicKeysValidator, TPublicKeysValidator, PUBLIC_KEY_SCHEMA_FOR_TRANSITION,
};

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = PublicKeysValidator)]
pub struct PublicKeysValidatorWasm {
    public_key_validator: PublicKeysValidator<BlsAdapter>,
    public_key_in_state_transition_validator: PublicKeysValidator<BlsAdapter>,
}

#[wasm_bindgen(js_class = PublicKeysValidator)]
impl PublicKeysValidatorWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(adapter: JsBlsAdapter) -> Result<PublicKeysValidatorWasm, JsError> {
        Ok(Self {
            public_key_validator: PublicKeysValidator::new(BlsAdapter(adapter.clone()))?,
            public_key_in_state_transition_validator: PublicKeysValidator::new_with_schema(
                PUBLIC_KEY_SCHEMA_FOR_TRANSITION.clone(),
                BlsAdapter(adapter),
            )?,
        })
    }

    #[wasm_bindgen(js_name=validateKeys)]
    pub fn validate_keys(
        &self,
        public_keys: js_sys::Array,
    ) -> Result<ValidationResultWasm, JsValue> {
        let raw_public_keys = to_vec_of_platform_values(public_keys.iter())?;

        let validation_result = self
            .public_key_validator
            .validate_keys(&raw_public_keys)
            .map_err(|e| JsValue::from(e.to_string()))?;
        Ok(validation_result.map(|_| JsValue::undefined()).into())
    }

    #[wasm_bindgen(js_name=validatePublicKeyStructure)]
    pub fn validate_public_key_structure(
        &self,
        public_key: JsValue,
    ) -> Result<ValidationResultWasm, JsValue> {
        let pk_object = public_key.with_serde_to_platform_value()?;

        let validation_result = self
            .public_key_validator
            .validate_public_key_structure(&pk_object)
            .map_err(|e| JsValue::from(e.to_string()))?;
        Ok(validation_result.map(|_| JsValue::undefined()).into())
    }

    #[wasm_bindgen(js_name=validateKeysInStateTransition)]
    pub fn validate_keys_in_state_transition(
        &self,
        public_keys: js_sys::Array,
    ) -> Result<ValidationResultWasm, JsValue> {
        let raw_public_keys = to_vec_of_platform_values(public_keys.iter())?;

        let validation_result = self
            .public_key_in_state_transition_validator
            .validate_keys(&raw_public_keys)
            .map_err(|e| JsValue::from(e.to_string()))?;
        Ok(validation_result.map(|_| JsValue::undefined()).into())
    }
}
