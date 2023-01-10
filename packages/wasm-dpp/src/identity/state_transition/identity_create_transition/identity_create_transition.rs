use std::default::Default;

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::identifier::IdentifierWrapper;
use crate::identity::state_transition::AssetLockProofWasm;

use crate::{
    buffer::Buffer,
    create_asset_lock_proof_from_wasm_instance,
    errors::RustConversionError,
    identity::{
        state_transition::asset_lock_proof::{ChainAssetLockProofWasm, InstantAssetLockProofWasm},
        IdentityPublicKeyWasm,
    },
    state_transition::StateTransitionExecutionContextWasm,
    with_js_error,
};

use crate::utils::ToSerdeJSONExt;
use dpp::{
    identifier::Identifier,
    identity::{
        state_transition::{
            asset_lock_proof::AssetLockProof, identity_create_transition::IdentityCreateTransition,
        },
        IdentityPublicKey,
    },
    state_transition::StateTransitionLike,
    util::string_encoding,
    util::string_encoding::Encoding,
};

#[wasm_bindgen(js_name=IdentityCreateTransition)]
#[derive(Clone)]
pub struct IdentityCreateTransitionWasm(IdentityCreateTransition);

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct IdentityCreateTransitionParams {
    public_keys: Vec<IdentityPublicKey>,
    signature: Option<Vec<u8>>,
    protocol_version: u32,
}

impl From<IdentityCreateTransition> for IdentityCreateTransitionWasm {
    fn from(v: IdentityCreateTransition) -> Self {
        IdentityCreateTransitionWasm(v)
    }
}

impl From<IdentityCreateTransitionWasm> for IdentityCreateTransition {
    fn from(v: IdentityCreateTransitionWasm) -> Self {
        v.0
    }
}

#[wasm_bindgen(js_class = IdentityCreateTransition)]
impl IdentityCreateTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(raw_parameters: JsValue) -> Result<IdentityCreateTransitionWasm, JsValue> {
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
        let asset_lock_proof = create_asset_lock_proof_from_wasm_instance(&asset_lock_proof)?;

        self.0
            .set_asset_lock_proof(asset_lock_proof)
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
            AssetLockProof::Chain(chain_asset_lock_proof) => {
                ChainAssetLockProofWasm::from(chain_asset_lock_proof).into()
            }
            AssetLockProof::Instant(instant_asset_lock_proof) => {
                InstantAssetLockProofWasm::from(instant_asset_lock_proof).into()
            }
        }
    }

    #[wasm_bindgen(js_name=setPublicKeys)]
    pub fn set_public_keys(&mut self, public_keys: js_sys::Array) -> Result<(), JsValue> {
        let public_keys = public_keys
            .to_vec()
            .into_iter()
            .map(|key| key.to_serde_json_value().unwrap())
            .map(|key_json| IdentityPublicKey::from_json_object(key_json).unwrap())
            .collect::<Vec<IdentityPublicKey>>();

        self.0.set_public_keys(public_keys);

        // TODO: consider returning self as it's done in the internal set_public_keys method
        Ok(())
    }

    #[wasm_bindgen(js_name=addPublicKeys)]
    pub fn add_public_keys(&mut self, public_keys: js_sys::Array) -> Result<(), JsValue> {
        let mut public_keys = public_keys
            .to_vec()
            .into_iter()
            .map(|key| key.to_serde_json_value().unwrap())
            .map(|key_json| IdentityPublicKey::from_json_object(key_json).unwrap())
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
        self.0.get_type() as u8
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
        let opts: super::to_object::ToObjectOptions = if options.is_object() {
            with_js_error!(serde_wasm_bindgen::from_value(options))?
        } else {
            Default::default()
        };

        let skip_signature = opts.skip_signature.clone();
        let object = super::to_object::to_object_struct(&self.0, opts);
        let js_object = js_sys::Object::new();

        js_sys::Reflect::set(
            &js_object,
            &"type".to_owned().into(),
            &object.transition_type.into(),
        )?;

        js_sys::Reflect::set(
            &js_object,
            &"protocolVersion".to_owned().into(),
            &object.protocol_version.into(),
        )?;

        if let Some(signature) = object.signature {
            let signature_value: JsValue = if signature.len() == 0 {
                JsValue::undefined()
            } else {
                Buffer::from_bytes(signature.as_slice()).into()
            };

            js_sys::Reflect::set(&js_object, &"signature".to_owned().into(), &signature_value)?;
        }

        js_sys::Reflect::set(
            &js_object,
            &"identityId".to_owned().into(),
            &Buffer::from_bytes(object.identity_id.buffer.as_slice()),
        )?;

        let asset_lock_proof_object = match object.asset_lock_proof {
            AssetLockProof::Instant(instant_asset_lock_proof) => {
                InstantAssetLockProofWasm::from(instant_asset_lock_proof).to_object()?
            }
            AssetLockProof::Chain(chain_asset_lock_proof) => {
                ChainAssetLockProofWasm::from(chain_asset_lock_proof).to_object()?
            }
        };

        js_sys::Reflect::set(
            &js_object,
            &"assetLockProof".to_owned().into(),
            &asset_lock_proof_object,
        )?;

        let keys_objects = object
            .public_keys
            .into_iter()
            .map(IdentityPublicKeyWasm::from)
            .map(|key| key.to_object(skip_signature))
            .collect::<Result<js_sys::Array, _>>()?;

        js_sys::Reflect::set(&js_object, &"publicKeys".to_owned().into(), &keys_objects)?;

        Ok(js_object.into())
    }

    #[wasm_bindgen(js_name=toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        let object = super::to_object::to_object_struct(&self.0, Default::default());
        let js_object = js_sys::Object::new();

        js_sys::Reflect::set(
            &js_object,
            &"type".to_owned().into(),
            &object.transition_type.into(),
        )?;

        js_sys::Reflect::set(
            &js_object,
            &"protocolVersion".to_owned().into(),
            &object.protocol_version.into(),
        )?;

        if let Some(signature) = object.signature {
            let signature_value: JsValue = if signature.len() == 0 {
                JsValue::undefined()
            } else {
                string_encoding::encode(signature.as_slice(), Encoding::Base64).into()
            };

            js_sys::Reflect::set(&js_object, &"signature".to_owned().into(), &signature_value)?;
        }

        js_sys::Reflect::set(
            &js_object,
            &"identityId".to_owned().into(),
            &object.identity_id.to_string(Encoding::Base58).into(),
        )?;

        let asset_lock_proof_json = match object.asset_lock_proof {
            AssetLockProof::Instant(instant_asset_lock_proof) => {
                InstantAssetLockProofWasm::from(instant_asset_lock_proof).to_json()?
            }
            AssetLockProof::Chain(chain_asset_lock_proof) => {
                ChainAssetLockProofWasm::from(chain_asset_lock_proof).to_json()?
            }
        };

        js_sys::Reflect::set(
            &js_object,
            &"assetLockProof".to_owned().into(),
            &asset_lock_proof_json,
        )?;

        let keys_objects = object
            .public_keys
            .into_iter()
            .map(IdentityPublicKeyWasm::from)
            .map(|key| key.to_json())
            .collect::<Result<js_sys::Array, _>>()?;

        js_sys::Reflect::set(&js_object, &"publicKeys".to_owned().into(), &keys_objects)?;

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

    #[wasm_bindgen(js_name=isDataContractStateTransition)]
    pub fn is_data_contract_state_transition(&self) -> bool {
        self.0.is_data_contract_state_transition()
    }

    #[wasm_bindgen(js_name=isDocumentStateTransition)]
    pub fn is_document_state_transition(&self) -> bool {
        self.0.is_document_state_transition()
    }

    #[wasm_bindgen(js_name=isIdentityStateTransition)]
    pub fn is_identity_state_transition(&self) -> bool {
        self.0.is_identity_state_transition()
    }

    #[wasm_bindgen(js_name=setExecutionContext)]
    pub fn set_execution_context(&mut self, context: &StateTransitionExecutionContextWasm) {
        self.0.set_execution_context(context.into())
    }
}
