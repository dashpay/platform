use crate::errors::RustConversionError;

use crate::identity::state_transition::identity_public_key_transitions::IdentityPublicKeyWithWitnessWasm;
use crate::identity::IdentityPublicKeyWasm;
use crate::utils::{generic_of_js_val, to_vec_of_platform_values};
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;

use dpp::identity::{IdentityPublicKey, KeyID};
use dpp::serialization::ValueConvertible;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use std::collections::BTreeMap;
use wasm_bindgen::__rt::Ref;
use wasm_bindgen::{JsCast, JsValue};

pub fn parse_public_keys(
    public_keys: js_sys::Array,
) -> Result<BTreeMap<KeyID, IdentityPublicKey>, JsValue> {
    let raw_public_keys = to_vec_of_platform_values(public_keys.iter())?;

    let public_keys = raw_public_keys
        .into_iter()
        .map(|v| IdentityPublicKey::from_object(v).map(|key| (key.id(), key)))
        .collect::<Result<_, _>>()
        .map_err(|e| format!("converting to collection of IdentityPublicKeys failed: {e:#}"))?;

    Ok(public_keys)
}

type AddPublicKeys = Option<Vec<IdentityPublicKeyInCreation>>;
type DisablePublicKeys = Option<Vec<KeyID>>;

pub fn parse_create_identity_update_transition_keys(
    public_keys: &JsValue,
) -> Result<(AddPublicKeys, DisablePublicKeys), JsValue> {
    let mut add_public_keys = None;

    if js_sys::Reflect::has(public_keys, &"add".into()).unwrap_or(false) {
        let raw_add_public_keys = js_sys::Reflect::get(public_keys, &"add".into()).unwrap();

        let add_public_keys_array: &js_sys::Array = raw_add_public_keys
            .dyn_ref::<js_sys::Array>()
            .ok_or_else(|| {
                RustConversionError::Error(String::from("public keys to add must be array"))
                    .to_js_value()
            })?;

        let keys: Vec<IdentityPublicKeyInCreation> = add_public_keys_array
            .iter()
            .map(|key| {
                let public_key: Ref<IdentityPublicKeyWithWitnessWasm> =
                    generic_of_js_val::<IdentityPublicKeyWithWitnessWasm>(
                        &key,
                        "IdentityPublicKeyWithWitness",
                    )?;

                Ok(public_key.clone().into())
            })
            .collect::<Result<Vec<IdentityPublicKeyInCreation>, JsValue>>()?;

        add_public_keys = Some(keys)
    }

    let mut disable_public_keys = None;

    if js_sys::Reflect::has(public_keys, &"disable".into()).unwrap_or(false) {
        let raw_disable_public_keys = js_sys::Reflect::get(public_keys, &"disable".into()).unwrap();
        let disable_public_keys_array: &js_sys::Array = raw_disable_public_keys
            .dyn_ref::<js_sys::Array>()
            .ok_or_else(|| {
                RustConversionError::Error(String::from("public keys to disable must be array"))
                    .to_js_value()
            })?;

        let keys: Vec<KeyID> = disable_public_keys_array
            .iter()
            .map(|key| {
                let public_key_wasm: Ref<IdentityPublicKeyWasm> =
                    generic_of_js_val::<IdentityPublicKeyWasm>(&key, "IdentityPublicKey")?;
                Ok(public_key_wasm.get_id())
            })
            .collect::<Result<Vec<KeyID>, JsValue>>()?;

        disable_public_keys = Some(keys)
    }

    Ok((add_public_keys, disable_public_keys))
}
