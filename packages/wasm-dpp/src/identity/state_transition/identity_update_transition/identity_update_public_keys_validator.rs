use crate::errors::from_dpp_err;
use crate::identity::state_transition::identity_public_key_transitions::IdentityPublicKeyCreateTransitionWasm;
use crate::validation::ValidationResultWasm;
use dpp::document::document_transition::document_base_transition::JsonValue;
use dpp::identity::state_transition::identity_public_key_transitions::IdentityPublicKeyCreateTransition;
use dpp::identity::validation::TPublicKeysValidator;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use dpp::identity::state_transition::identity_update_transition::validation::state::validate_public_keys::IdentityUpdatePublicKeysValidator;

#[wasm_bindgen(js_name=IdentityUpdatePublicKeysValidator)]
pub struct IdentityUpdatePublicKeysValidatorWasm(IdentityUpdatePublicKeysValidator);

impl From<IdentityUpdatePublicKeysValidator> for IdentityUpdatePublicKeysValidatorWasm {
    fn from(v: IdentityUpdatePublicKeysValidator) -> Self {
        Self(v)
    }
}

#[wasm_bindgen(js_class=IdentityUpdatePublicKeysValidator)]
impl IdentityUpdatePublicKeysValidatorWasm {
    #[wasm_bindgen(constructor)]
    pub fn new() -> IdentityUpdatePublicKeysValidatorWasm {
        let validator = IdentityUpdatePublicKeysValidator {};
        validator.into()
    }

    #[wasm_bindgen]
    pub fn validate(&self, raw_public_keys: Vec<JsValue>) -> Result<ValidationResultWasm, JsValue> {
        let public_keys = raw_public_keys
            .into_iter()
            .map(|raw_key| {
                let parsed_key: IdentityPublicKeyCreateTransition =
                    IdentityPublicKeyCreateTransitionWasm::new(raw_key)?.into();

                parsed_key
                    .to_raw_json_object(false)
                    .map_err(|e| from_dpp_err(e.into()))
            })
            .collect::<Result<Vec<JsonValue>, JsValue>>()?;

        let result = self
            .0
            .validate_keys(&public_keys)
            .map_err(|e| from_dpp_err(e.into()))?;

        Ok(result.map(|_| JsValue::undefined()).into())
    }
}

impl Default for IdentityUpdatePublicKeysValidatorWasm {
    fn default() -> Self {
        Self::new()
    }
}
