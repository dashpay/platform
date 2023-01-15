use crate::bls_adapter::{BlsAdapter, JsBlsAdapter};
use crate::errors::from_dpp_err;
use crate::IdentityCreateTransitionWasm;
use crate::{
    identity::identity_public_key::IdentityPublicKeyWasm, validation::ValidationResultWasm,
};
use dpp::identity::state_transition::identity_create_transition::IdentityCreateTransition;
use dpp::identity::state_transition::validate_public_key_signatures::{
    PublicKeysSignaturesValidator, TPublicKeysSignaturesValidator,
};
use dpp::prelude::IdentityPublicKey;
use dpp::state_transition::{StateTransitionConvert, StateTransitionType};
use serde_json::Value as JsonValue;
use std::convert::TryInto;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name=validatePublicKeySignatures)]
pub fn validate_public_key_signatures(
    raw_state_transition: JsValue,
    raw_public_keys: Vec<JsValue>,
    bls: JsBlsAdapter,
) -> Result<ValidationResultWasm, JsValue> {
    // TODO: use with_serde_to_json_value?
    let st_type: StateTransitionType =
        (js_sys::Reflect::get(&raw_state_transition, &JsValue::from_str("type"))?
            .as_f64()
            .ok_or(JsValue::from_str("State transition type is not a number"))? as u8)
            .try_into()
            .map_err(|_| js_sys::Error::new("State transition type is not a valid value"))?;

    let bls_adapter = BlsAdapter(bls);

    let st_object = match st_type {
        StateTransitionType::IdentityCreate => {
            let st: IdentityCreateTransition =
                IdentityCreateTransitionWasm::new(raw_state_transition)?.into();

            st.to_object(true).map_err(|e| from_dpp_err(e.into()))
        }
        // TODO:
        // StateTransitionType::IdentityUpdate => {
        //     let st: IdentityUpdateTransition =
        //         IdentityUpdateTransitionWasm::new(raw_state_transition)?.into();
        //     st.to_json(false)
        //         .map_err(from_dpp_err)
        // }
        _ => Err(js_sys::Error::new("State transition type is not supported").into()),
    }?;

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
        .validate_public_key_signatures(&st_object, &public_keys)
        .map_err(|e| from_dpp_err(e.into()))?;

    Ok(result.map(|_| JsValue::undefined()).into())
}
