use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::identifier::IdentifierWrapper;
use crate::state_transition::AssetLockProofWasm;
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

    #[wasm_bindgen(js_name=setAssetLockProof)]
    pub fn set_asset_lock_proof(&mut self, asset_lock_proof: JsValue) -> Result<(), JsValue> {
        let asset_lock_proof = AssetLockProofWasm::new(asset_lock_proof)?;

        self.0
            .set_asset_lock_proof(asset_lock_proof.into())
            .map_err(|e| RustConversionError::Error(e.to_string()).to_js_value())?;

        Ok(())
    }

    #[wasm_bindgen(getter, js_name=assetLockProof)]
    pub fn asset_lock_proof(&self) -> JsValue {
        self.get_asset_lock_proof()
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

    #[wasm_bindgen(js_name=setPublicKeys)]
    pub fn set_public_keys(&mut self, public_keys: Vec<JsValue>) -> Result<(), JsValue> {
        let public_keys = public_keys
            .into_iter()
            .map(|key| IdentityPublicKeyWasm::new(key))
            .collect::<Result<Vec<IdentityPublicKeyWasm>, _>>()?;

        self.0
            .set_public_keys(public_keys.into_iter().map(|key| key.into()).collect());

        // TODO: consider returning self as it's done in the internal set_public_keys method
        Ok(())
    }

    #[wasm_bindgen(js_name=addPublicKeys)]
    pub fn add_public_keys(&mut self, public_keys: Vec<JsValue>) -> Result<(), JsValue> {
        let public_keys_wasm: Vec<IdentityPublicKeyWasm> = public_keys
            .into_iter()
            .map(|key| IdentityPublicKeyWasm::new(key))
            .collect::<Result<Vec<IdentityPublicKeyWasm>, _>>()?;

        let mut public_keys = public_keys_wasm
            .into_iter()
            .map(|key| key.into())
            .collect::<Vec<IdentityPublicKey>>();

        self.0.add_public_keys(&mut public_keys);

        // TODO: consider returning self as it's done in the internal add_public_keys method
        Ok(())
    }

    #[wasm_bindgen(js_name=getPublicKeys)]
    pub fn get_public_keys(&self) -> Vec<JsValue> {
        self.0
            .get_public_keys()
            .iter()
            .map(IdentityPublicKey::to_owned)
            .map(IdentityPublicKeyWasm::from)
            .map(JsValue::from)
            .collect()
    }

    #[wasm_bindgen(getter, js_name=publicKeys)]
    pub fn public_keys(&self) -> Vec<JsValue> {
        self.get_public_keys()
    }

    #[wasm_bindgen(js_name=getType)]
    pub fn get_type(&self) -> u8 {
        IdentityCreateTransition::get_type() as u8
    }

    #[wasm_bindgen(getter, js_name=identityId)]
    pub fn identity_id(&self) -> IdentifierWrapper {
        self.get_identity_id()
    }

    #[wasm_bindgen(js_name=getIdentityId)]
    pub fn get_identity_id(&self) -> IdentifierWrapper {
        self.0.get_identity_id().clone().into()
    }

    #[wasm_bindgen(js_name=getOwnerId)]
    pub fn get_owner_id(&self) -> IdentifierWrapper {
        self.0.get_owner_id().clone().into()
    }

    // pub fn to_object(&self) -> JsValue {
    // let json_object = self.0.to_json_object()
    // serde_wasm_bindgen::to_value(&self.0).unwrap()
    // }
}
