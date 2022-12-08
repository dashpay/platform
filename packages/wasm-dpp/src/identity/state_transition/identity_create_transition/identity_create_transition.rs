use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::{
    errors::RustConversionError,
    identity::{
        state_transition::asset_lock_proof::{ChainAssetLockProofWasm, InstantAssetLockProofWasm},
        IdentityPublicKeyWasm,
    },
    with_js_error,
};
use dpp::identity::state_transition::asset_lock_proof::AssetLockProof;
use dpp::identity::{
    state_transition::identity_create_transition::IdentityCreateTransition, IdentityPublicKey,
};

#[wasm_bindgen(js_name=IdentityCreateTransition)]
pub struct IdentityCreateTransitionWasm(IdentityCreateTransition);

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct IdentityCreateTransitionParams {
    asset_lock_proof: InstantAssetLockProofWasm,
    public_keys: Vec<IdentityPublicKey>,
}

impl From<IdentityCreateTransition> for IdentityCreateTransitionWasm {
    fn from(v: IdentityCreateTransition) -> Self {
        IdentityCreateTransitionWasm(v)
    }
}

#[wasm_bindgen(js_class = IdentityCreateTransition)]
impl IdentityCreateTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(raw_parameters: JsValue) -> Result<IdentityCreateTransitionWasm, JsValue> {
        let parameters: IdentityCreateTransitionParams =
            with_js_error!(serde_wasm_bindgen::from_value(raw_parameters))?;

        let raw_state_transition = with_js_error!(serde_json::to_value(&parameters))?;

        let identity_create_transition = IdentityCreateTransition::new(raw_state_transition)
            .map_err(|e| RustConversionError::Error(e.to_string()).to_js_value())?;

        Ok(identity_create_transition.into())
    }

    #[wasm_bindgen(getter)]
    #[wasm_bindgen(js_name=publicKeys)]
    pub fn public_keys(&self) -> Vec<JsValue> {
        self.0
            .get_public_keys()
            .iter()
            .map(IdentityPublicKey::to_owned)
            .map(IdentityPublicKeyWasm::from)
            .map(JsValue::from)
            .collect()
    }

    #[wasm_bindgen(js_name=getAssetLockProof)]
    pub fn get_asset_lock_proof(&self) -> JsValue {
        let asset_lock_proof = self.0.get_asset_lock_proof().to_owned();
        match asset_lock_proof {
            AssetLockProof::Instant(instant_asset_lock_proof) => {
                InstantAssetLockProofWasm::from(instant_asset_lock_proof).into()
            }
            AssetLockProof::Chain(chain_asset_lock_proof) => {
                ChainAssetLockProofWasm::from(chain_asset_lock_proof).into()
            }
        }
    }
}
