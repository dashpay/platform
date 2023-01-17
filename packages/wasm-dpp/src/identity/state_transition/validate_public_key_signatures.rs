use crate::bls_adapter::{BlsAdapter, JsBlsAdapter};
use crate::errors::from_dpp_err;
use crate::utils::ToSerdeJSONExt;

use crate::{
    identity::identity_public_key::IdentityPublicKeyWasm, validation::ValidationResultWasm,
};

use dpp::identity::state_transition::validate_public_key_signatures::{
    PublicKeysSignaturesValidator, TPublicKeysSignaturesValidator,
};
use dpp::prelude::IdentityPublicKey;

use serde_json::Value as JsonValue;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name=validatePublicKeySignatures)]
pub fn validate_public_key_signatures(
    raw_state_transition: JsValue,
    raw_public_keys: Vec<JsValue>,
    bls: JsBlsAdapter,
) -> Result<ValidationResultWasm, JsValue> {
    let bls_adapter = BlsAdapter(bls);

    let state_transition_json = raw_state_transition.with_serde_to_json_value()?;

    let public_keys = raw_public_keys
        .into_iter()
        .map(|raw_key| {
            let parsed_key: IdentityPublicKey = IdentityPublicKeyWasm::new(raw_key)?.into();
            parsed_key
                .to_raw_json_object(false)
                .map_err(|e| from_dpp_err(e.into()))
        })
        .collect::<Result<Vec<JsonValue>, JsValue>>()?;

    let validator = PublicKeysSignaturesValidator::new(bls_adapter);
    let result = validator
        .validate_public_key_signatures(&state_transition_json, &public_keys)
        .map_err(|e| from_dpp_err(e.into()))?;

    Ok(result.map(|_| JsValue::undefined()).into())
}
