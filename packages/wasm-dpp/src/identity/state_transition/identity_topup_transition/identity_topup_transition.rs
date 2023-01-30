use crate::utils::ToSerdeJSONExt;

use std::convert::TryInto;
use std::default::Default;

use wasm_bindgen::prelude::*;

use crate::identifier::IdentifierWrapper;

use crate::{
    buffer::Buffer,
    create_asset_lock_proof_from_wasm_instance,
    errors::RustConversionError,
    identity::state_transition::asset_lock_proof::{
        ChainAssetLockProofWasm, InstantAssetLockProofWasm,
    },
    state_transition::StateTransitionExecutionContextWasm,
    with_js_error,
};

use crate::bls_adapter::{BlsAdapter, JsBlsAdapter};
use crate::errors::from_dpp_err;

use dpp::{
    identifier::Identifier,
    identity::state_transition::{
        asset_lock_proof::AssetLockProof, identity_topup_transition::IdentityTopUpTransition,
    },
    state_transition::StateTransitionLike,
    util::string_encoding,
    util::string_encoding::Encoding,
};

#[wasm_bindgen(js_name=IdentityTopUpTransition)]
#[derive(Clone)]
pub struct IdentityTopUpTransitionWasm(IdentityTopUpTransition);

impl From<IdentityTopUpTransition> for IdentityTopUpTransitionWasm {
    fn from(v: IdentityTopUpTransition) -> Self {
        IdentityTopUpTransitionWasm(v)
    }
}

impl From<IdentityTopUpTransitionWasm> for IdentityTopUpTransition {
    fn from(v: IdentityTopUpTransitionWasm) -> Self {
        v.0
    }
}

#[wasm_bindgen(js_class = IdentityTopUpTransition)]
impl IdentityTopUpTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(raw_parameters: JsValue) -> Result<IdentityTopUpTransitionWasm, JsValue> {
        let raw_state_transition = raw_parameters.with_serde_to_json_value()?;

        let identity_topup_transition =
            IdentityTopUpTransition::from_raw_object(raw_state_transition)
                .map_err(|e| RustConversionError::Error(e.to_string()).to_js_value())?;

        Ok(identity_topup_transition.into())
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
            let signature_value: JsValue = if signature.is_empty() {
                JsValue::undefined()
            } else {
                Buffer::from_bytes(signature.as_slice()).into()
            };

            js_sys::Reflect::set(&js_object, &"signature".to_owned().into(), &signature_value)?;
        }

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

        js_sys::Reflect::set(
            &js_object,
            &"identityId".to_owned().into(),
            &Buffer::from_bytes(object.identity_id.buffer.as_slice()),
        )?;

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
            let signature_value: JsValue = if signature.is_empty() {
                JsValue::undefined()
            } else {
                string_encoding::encode(signature.as_slice(), Encoding::Base64).into()
            };

            js_sys::Reflect::set(&js_object, &"signature".to_owned().into(), &signature_value)?;
        }

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

        let identity_id = object.identity_id.to_string(Encoding::Base58);

        js_sys::Reflect::set(
            &js_object,
            &"identityId".to_owned().into(),
            &identity_id.into(),
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

    #[wasm_bindgen(js_name=getExecutionContext)]
    pub fn get_execution_context(&mut self) -> StateTransitionExecutionContextWasm {
        self.0.get_execution_context().into()
    }

    #[wasm_bindgen(js_name=signByPrivateKey)]
    pub fn sign_by_private_key(
        &mut self,
        private_key: Vec<u8>,
        key_type: u8,
        bls: JsBlsAdapter,
    ) -> Result<(), JsValue> {
        let bls_adapter = BlsAdapter(bls);
        let key_type = key_type
            .try_into()
            .map_err(|e: anyhow::Error| e.to_string())?;

        self.0
            .sign_by_private_key(private_key.as_slice(), key_type, &bls_adapter)
            .map_err(from_dpp_err)
    }

    #[wasm_bindgen(js_name=getSignature)]
    pub fn get_signature(&self) -> Buffer {
        Buffer::from_bytes(self.0.get_signature().as_slice())
    }
}
