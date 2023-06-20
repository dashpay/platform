use crate::bls_adapter::{BlsAdapter, JsBlsAdapter};

use crate::utils::{ToSerdeJSONExt, WithJsError};

use crate::validation::ValidationResultWasm;

use crate::identity::state_transition::identity_public_key_transitions::IdentityPublicKeyWithWitnessWasm;

use dpp::identity::state_transition::validate_public_key_signatures::{
    PublicKeysSignaturesValidator, TPublicKeysSignaturesValidator,
};

use dpp::identity::state_transition::identity_public_key_transitions::IdentityPublicKeyInCreation;

use crate::errors::from_dpp_err;
use dpp::platform_value::Value;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name = PublicKeysSignaturesValidator)]
pub struct PublicKeysSignaturesValidatorWasm(PublicKeysSignaturesValidator<BlsAdapter>);

impl From<PublicKeysSignaturesValidator<BlsAdapter>> for PublicKeysSignaturesValidatorWasm {
    fn from(validator: PublicKeysSignaturesValidator<BlsAdapter>) -> Self {
        Self(validator)
    }
}

#[wasm_bindgen(js_class = PublicKeysSignaturesValidator)]
impl PublicKeysSignaturesValidatorWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(bls: JsBlsAdapter) -> PublicKeysSignaturesValidatorWasm {
        PublicKeysSignaturesValidator::new(BlsAdapter(bls)).into()
    }

    #[wasm_bindgen(js_name=validate)]
    pub fn validate_public_key_signatures(
        &self,
        raw_state_transition: JsValue,
        raw_public_keys: Vec<JsValue>,
    ) -> Result<ValidationResultWasm, JsValue> {
        let state_transition_object = raw_state_transition.with_serde_to_platform_value()?;

        let public_keys = raw_public_keys
            .into_iter()
            .map(|raw_key| {
                let parsed_key: IdentityPublicKeyInCreation =
                    IdentityPublicKeyWithWitnessWasm::new(raw_key)?.into();
                parsed_key.to_raw_object(false).with_js_error()
            })
            .collect::<Result<Vec<Value>, JsValue>>()?;

        let result = self
            .0
            .validate_public_key_signatures(&state_transition_object, &public_keys)
            .map_err(|e| from_dpp_err(e.into()))?;

        Ok(result.map(|_| JsValue::undefined()).into())
    }
}
