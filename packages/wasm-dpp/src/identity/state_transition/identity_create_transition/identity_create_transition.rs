use std::convert::TryInto;
use std::default::Default;

use wasm_bindgen::__rt::Ref;
use wasm_bindgen::prelude::*;

use crate::identifier::IdentifierWrapper;

use crate::{
    buffer::Buffer,
    create_asset_lock_proof_from_wasm_instance,
    errors::RustConversionError,
    identity::state_transition::asset_lock_proof::{
        ChainAssetLockProofWasm, InstantAssetLockProofWasm,
    },
    identity::state_transition::identity_public_key_transitions::IdentityPublicKeyWithWitnessWasm,
    with_js_error,
};

use crate::bls_adapter::{BlsAdapter, JsBlsAdapter};
use dpp::state_transition::StateTransition;

use crate::utils::{generic_of_js_val, ToSerdeJSONExt, WithJsError};
use dpp::identity::KeyType;
use dpp::platform_value::string_encoding::Encoding;
use dpp::platform_value::{string_encoding, BinaryData};
use dpp::serialization_traits::PlatformSerializable;
use dpp::{
    identity::state_transition::{
        asset_lock_proof::AssetLockProof, identity_create_transition::IdentityCreateTransition,
        identity_public_key_transitions::IdentityPublicKeyInCreation,
    },
    state_transition::StateTransitionLike,
};

#[wasm_bindgen(js_name=IdentityCreateTransition)]
#[derive(Clone)]
pub struct IdentityCreateTransitionWasm(IdentityCreateTransition);

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
        let mut raw_state_transition = raw_parameters.with_serde_to_platform_value()?;
        IdentityCreateTransition::clean_value(&mut raw_state_transition).with_js_error()?;
        let identity_create_transition =
            IdentityCreateTransition::from_raw_object(raw_state_transition)
                .map_err(|e| RustConversionError::Error(e.to_string()).to_js_value())?;

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
    pub fn set_public_keys(&mut self, public_keys: Vec<JsValue>) -> Result<(), JsValue> {
        let public_keys = public_keys
            .iter()
            .map(|value| {
                let public_key: Ref<IdentityPublicKeyWithWitnessWasm> =
                    generic_of_js_val::<IdentityPublicKeyWithWitnessWasm>(
                        value,
                        "IdentityPublicKeyWithWitness",
                    )?;
                Ok(public_key.clone().into())
            })
            .collect::<Result<Vec<IdentityPublicKeyInCreation>, JsValue>>()?;

        self.0.set_public_keys(public_keys);

        Ok(())
    }

    #[wasm_bindgen(js_name=addPublicKeys)]
    pub fn add_public_keys(&mut self, public_keys: Vec<JsValue>) -> Result<(), JsValue> {
        let mut public_keys = public_keys
            .iter()
            .map(|value| {
                let public_key: Ref<IdentityPublicKeyWithWitnessWasm> =
                    generic_of_js_val::<IdentityPublicKeyWithWitnessWasm>(
                        value,
                        "IdentityPublicKeyWithWitness",
                    )?;
                Ok(public_key.clone().into())
            })
            .collect::<Result<Vec<IdentityPublicKeyInCreation>, JsValue>>()?;

        self.0.add_public_keys(&mut public_keys);

        Ok(())
    }

    #[wasm_bindgen(js_name=getPublicKeys)]
    pub fn get_public_keys(&self) -> Vec<JsValue> {
        self.0
            .get_public_keys()
            .iter()
            .map(IdentityPublicKeyInCreation::to_owned)
            .map(IdentityPublicKeyWithWitnessWasm::from)
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
        (*self.0.get_identity_id()).into()
    }

    #[wasm_bindgen(js_name=getOwnerId)]
    pub fn get_owner_id(&self) -> IdentifierWrapper {
        (*self.0.get_owner_id()).into()
    }

    #[wasm_bindgen(js_name=toObject)]
    pub fn to_object(&self, options: JsValue) -> Result<JsValue, JsValue> {
        let opts: super::to_object::ToObjectOptions = if options.is_object() {
            with_js_error!(serde_wasm_bindgen::from_value(options.clone()))?
        } else {
            Default::default()
        };

        let _skip_signature = opts.skip_signature;
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
                Buffer::from_bytes(&signature).into()
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

        let keys_objects = object
            .public_keys
            .into_iter()
            .map(IdentityPublicKeyWithWitnessWasm::from)
            .map(|key| key.to_object(options.clone()))
            .collect::<Result<js_sys::Array, _>>()?;

        js_sys::Reflect::set(&js_object, &"publicKeys".to_owned().into(), &keys_objects)?;

        Ok(js_object.into())
    }

    #[wasm_bindgen(js_name=toBuffer)]
    pub fn to_buffer(&self) -> Result<Buffer, JsValue> {
        let bytes =
            PlatformSerializable::serialize(&StateTransition::IdentityCreate(self.0.clone()))
                .with_js_error()?;
        Ok(Buffer::from_bytes(&bytes))
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

        let keys_objects = object
            .public_keys
            .into_iter()
            .map(IdentityPublicKeyWithWitnessWasm::from)
            .map(|key| key.to_json())
            .collect::<Result<js_sys::Array, _>>()?;

        js_sys::Reflect::set(&js_object, &"publicKeys".to_owned().into(), &keys_objects)?;

        Ok(js_object.into())
    }

    #[wasm_bindgen(js_name=getModifiedDataIds)]
    pub fn get_modified_data_ids(&self) -> Vec<IdentifierWrapper> {
        let ids = self.0.get_modified_data_ids();

        ids.into_iter().map(IdentifierWrapper::from).collect()
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

    #[wasm_bindgen(js_name=signByPrivateKey)]
    pub fn sign_by_private_key(
        &mut self,
        private_key: Vec<u8>,
        key_type: u8,
        bls: Option<JsBlsAdapter>,
    ) -> Result<(), JsValue> {
        let key_type = key_type
            .try_into()
            .map_err(|e: anyhow::Error| e.to_string())?;

        if bls.is_none() && key_type == KeyType::BLS12_381 {
            return Err(JsError::new(
                format!("BLS adapter is required for BLS key type '{}'", key_type).as_str(),
            )
            .into());
        }

        let bls_adapter = if let Some(adapter) = bls {
            BlsAdapter(adapter)
        } else {
            BlsAdapter(JsValue::undefined().into())
        };

        self.0
            .sign_by_private_key(private_key.as_slice(), key_type, &bls_adapter)
            .with_js_error()
    }

    #[wasm_bindgen(js_name=getSignature)]
    pub fn get_signature(&self) -> Buffer {
        Buffer::from_bytes_owned(self.0.get_signature().to_vec())
    }
    #[wasm_bindgen(js_name=setSignature)]
    pub fn set_signature(&mut self, signature: Option<Vec<u8>>) {
        self.0.signature = BinaryData::new(signature.unwrap_or(vec![]))
    }
}
