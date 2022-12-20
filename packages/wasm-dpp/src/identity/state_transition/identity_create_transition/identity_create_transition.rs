use std::default::Default;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use wasm_bindgen::prelude::*;

use crate::identifier::IdentifierWrapper;
use crate::state_transition::AssetLockProofWasm;
use crate::{
    buffer::Buffer,
    errors::RustConversionError,
    identity::{
        state_transition::asset_lock_proof::{ChainAssetLockProofWasm, InstantAssetLockProofWasm},
        IdentityPublicKeyWasm,
    },
    with_js_error,
};

use dpp::{
    identifier::Identifier,
    identity::{
        state_transition::{
            asset_lock_proof::AssetLockProof,
            identity_create_transition::{IdentityCreateTransition, SerializationOptions},
        },
        IdentityPublicKey,
    },
    state_transition::StateTransitionLike,
    util::string_encoding,
    util::string_encoding::Encoding,
};

#[wasm_bindgen(js_name=IdentityCreateTransition)]
pub struct IdentityCreateTransitionWasm(IdentityCreateTransition);

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SerializationOptionsJS {
    // TODO: remove JS
    pub skip_signature: Option<bool>,
    pub skip_identifiers_conversion: Option<bool>,
}

// TODO: remove?
impl From<SerializationOptionsJS> for SerializationOptions {
    fn from(options: SerializationOptionsJS) -> Self {
        Self {
            skip_signature: options.skip_signature.unwrap_or(false),
            skip_identifiers_conversion: options.skip_identifiers_conversion.unwrap_or(false),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawInstantAssetLockProof {
    #[serde(rename = "type")]
    lock_type: u8,
    instant_lock: Vec<u8>,
    transaction: Vec<u8>,
    output_index: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawChainAssetLockProof {
    #[serde(rename = "type")]
    lock_type: u8,
    core_chain_locked_height: u32,
    out_point: Vec<u8>,
}

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
pub enum RawAssetLockProof {
    Instant(RawInstantAssetLockProof),
    Chain(RawChainAssetLockProof),
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct IdentityCreateTransitionParams {
    // Omitting asset lock proof because it gets parsed separately depending on it's type
    public_keys: Vec<IdentityPublicKey>,
    signature: Option<Vec<u8>>,
    // Add protocol version
}

impl From<IdentityCreateTransition> for IdentityCreateTransitionWasm {
    fn from(v: IdentityCreateTransition) -> Self {
        IdentityCreateTransitionWasm(v)
    }
}

#[wasm_bindgen(js_class = IdentityCreateTransition)]
impl IdentityCreateTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(mut raw_parameters: JsValue) -> Result<IdentityCreateTransitionWasm, JsValue> {
        let raw_asset_lock_proof =
            js_sys::Reflect::get(&raw_parameters, &"assetLockProof".to_owned().into())?;

        let parameters: IdentityCreateTransitionParams =
            with_js_error!(serde_wasm_bindgen::from_value(raw_parameters))?;

        let raw_state_transition = with_js_error!(serde_json::to_value(&parameters))?;

        let mut identity_create_transition = IdentityCreateTransition::new(raw_state_transition)
            .map_err(|e| RustConversionError::Error(e.to_string()).to_js_value())?;

        let asset_lock_proof = AssetLockProofWasm::new(raw_asset_lock_proof)?;
        identity_create_transition
            .set_asset_lock_proof(asset_lock_proof.into())
            .map_err(|e| RustConversionError::Error(e.to_string()).to_js_value())?;

        if let Some(signature) = parameters.signature {
            identity_create_transition.set_signature(signature);
        }

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

    #[wasm_bindgen(js_name=toObject)]
    pub fn to_object(&self, options: JsValue) -> Result<JsValue, JsValue> {
        let opts: SerializationOptionsJS = if options.is_object() {
            with_js_error!(serde_wasm_bindgen::from_value(options))?
        } else {
            Default::default()
        };

        let js_object = js_sys::Object::new();

        // Add signature
        if !opts.skip_signature.unwrap_or(false) {
            let signature = self.0.get_signature();
            let signature_buffer = Buffer::from_bytes(signature.as_slice());

            js_sys::Reflect::set(
                &js_object,
                &"signature".to_owned().into(),
                &signature_buffer.into(),
            )?;
        }

        // Add identityId (following to_json_object example if rs-dpp IdentityCreateTransition)
        if !opts.skip_identifiers_conversion.unwrap_or(false) {
            let signature = self.0.get_signature();
            let signature_buffer = Buffer::from_bytes(signature.as_slice());
            js_sys::Reflect::set(
                &js_object,
                &"identityId".to_owned().into(),
                &signature_buffer.into(),
            )?;
        } else {
            js_sys::Reflect::set(
                &js_object,
                &"identityId".to_owned().into(),
                &self.get_identity_id().into(),
            )?;
        }

        // Write asset lock proof wasm object
        let asset_lock_proof = self.get_asset_lock_proof();
        js_sys::Reflect::set(
            &js_object,
            &"assetLockProof".to_owned().into(),
            &asset_lock_proof.into(),
        )?;

        // Write array of public keys
        let public_keys = self.get_public_keys();
        let js_public_keys = js_sys::Array::new();

        for pk in public_keys {
            js_public_keys.push(&pk);
        }
        js_sys::Reflect::set(
            &js_object,
            &"publicKeys".to_owned().into(),
            &JsValue::from(&js_public_keys),
        )?;

        // Write ST type
        let transition_type = self.get_type();
        js_sys::Reflect::set(
            &js_object,
            &"type".to_owned().into(),
            &JsValue::from(transition_type),
        )?;

        Ok(js_object.into())
    }

    #[wasm_bindgen(js_name=toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        let js_object = js_sys::Object::new();

        // Write signature
        let signature = self.0.get_signature();
        let signature_base64 = string_encoding::encode(signature.as_slice(), Encoding::Base64);

        js_sys::Reflect::set(
            &js_object,
            &"signature".to_owned().into(),
            &JsValue::from(&signature_base64),
        )?;

        // Write identityId (following to_json_object example if rs-dpp IdentityCreateTransition)
        js_sys::Reflect::set(
            &js_object,
            &"identityId".to_owned().into(),
            &JsValue::from(&signature_base64),
        )?;

        // Write asset lock proof JSON
        let asset_lock_proof = self.0.get_asset_lock_proof().to_owned();
        let asset_lock_proof_json = match asset_lock_proof {
            AssetLockProof::Instant(instant_asset_lock_proof) => {
                InstantAssetLockProofWasm::from(instant_asset_lock_proof).to_json()
            }
            AssetLockProof::Chain(chain_asset_lock_proof) => {
                ChainAssetLockProofWasm::from(chain_asset_lock_proof).to_json()
            }
        }?;

        js_sys::Reflect::set(
            &js_object,
            &"assetLockProof".to_owned().into(),
            &asset_lock_proof_json,
        )?;

        // Write public keys JSON values
        let public_keys: Vec<JsValue> = self
            .0
            .get_public_keys()
            .iter()
            .map(IdentityPublicKey::to_owned)
            .map(|key| IdentityPublicKeyWasm::from(key).to_json().ok())
            .map(JsValue::from)
            .collect();

        let js_public_keys = js_sys::Array::new();

        for pk in public_keys {
            js_public_keys.push(&pk);
        }
        js_sys::Reflect::set(
            &js_object,
            &"publicKeys".to_owned().into(),
            &JsValue::from(&js_public_keys),
        )?;

        // Write type value
        let transition_type = self.get_type();
        js_sys::Reflect::set(
            &js_object,
            &"type".to_owned().into(),
            &JsValue::from(transition_type),
        )?;

        Ok(js_object.into())
    }

    #[wasm_bindgen(js_name=getModifiedDataIds)]
    pub fn get_modified_data_ids(&self) -> Vec<JsValue> {
        let ids = self.0.get_modified_data_ids();

        ids.into_iter()
            .map(|id| {
                <IdentifierWrapper as std::convert::From<Identifier>>::from(id.clone()).into()
            })
            .collect()
    }
}
