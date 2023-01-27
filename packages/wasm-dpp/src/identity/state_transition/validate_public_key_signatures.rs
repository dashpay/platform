use crate::bls_adapter::{BlsAdapter, JsBlsAdapter};
use crate::errors::from_dpp_err;
use crate::utils::ToSerdeJSONExt;

use crate::{
    identity::identity_public_key::IdentityPublicKeyWasm, validation::ValidationResultWasm,
    IdentityPublicKeyInCreationWasm,
};

use dpp::identity::state_transition::validate_public_key_signatures::{
    PublicKeysSignaturesValidator, TPublicKeysSignaturesValidator,
};
use dpp::prelude::IdentityPublicKey;

use dpp::identity::IdentityPublicKeyInCreation;
use serde_json::Value as JsonValue;
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
        let state_transition_json = raw_state_transition.with_serde_to_json_value()?;

        let public_keys = raw_public_keys
            .into_iter()
            .map(|raw_key| {
                let parsed_key: IdentityPublicKeyInCreation =
                    IdentityPublicKeyInCreationWasm::new(raw_key)?.into();
                parsed_key
                    .to_raw_json_object()
                    .map_err(|e| from_dpp_err(e.into()))
            })
            .collect::<Result<Vec<JsonValue>, JsValue>>()?;

        let result = self
            .0
            .validate_public_key_signatures(&state_transition_json, &public_keys)
            .map_err(|e| from_dpp_err(e.into()))?;

        Ok(result.map(|_| JsValue::undefined()).into())
    }
}
