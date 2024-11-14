use std::convert::TryInto;
use std::default::Default;

use serde::{Deserialize, Serialize};

use wasm_bindgen::__rt::Ref;
use wasm_bindgen::prelude::*;

use crate::identifier::IdentifierWrapper;

use crate::{
    buffer::Buffer,
    identity::state_transition::identity_public_key_transitions::IdentityPublicKeyWithWitnessWasm,
    identity::IdentityPublicKeyWasm, with_js_error,
};

use crate::bls_adapter::{BlsAdapter, JsBlsAdapter};

use crate::utils::{generic_of_js_val, WithJsError};

use crate::errors::from_dpp_err;
use dpp::errors::consensus::signature::SignatureError;
use dpp::errors::consensus::ConsensusError;
use dpp::errors::ProtocolError;
use dpp::identity::{KeyID, KeyType, TimestampMillis};
use dpp::platform_value::string_encoding::Encoding;
use dpp::platform_value::{string_encoding, BinaryData};
use dpp::prelude::Revision;
use dpp::serialization::PlatformSerializable;
use dpp::state_transition::identity_update_transition::accessors::IdentityUpdateTransitionAccessorsV0;
use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::state_transition::StateTransition;
use dpp::state_transition::StateTransitionIdentitySigned;
use dpp::version::PlatformVersion;
use dpp::{identifier::Identifier, state_transition::StateTransitionLike};

#[wasm_bindgen(js_name=IdentityUpdateTransition)]
#[derive(Clone)]
pub struct IdentityUpdateTransitionWasm(IdentityUpdateTransition);

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct IdentityUpdateTransitionParams {
    signature: Vec<u8>,
    signature_public_key_id: KeyID,
    protocol_version: u32,
    identity_id: Vec<u8>,
    revision: Revision,
    add_public_keys: Option<Vec<IdentityPublicKeyInCreation>>,
    disable_public_keys: Option<Vec<KeyID>>,
    public_keys_disabled_at: Option<TimestampMillis>,
}

impl From<IdentityUpdateTransition> for IdentityUpdateTransitionWasm {
    fn from(v: IdentityUpdateTransition) -> Self {
        IdentityUpdateTransitionWasm(v)
    }
}

impl From<IdentityUpdateTransitionWasm> for IdentityUpdateTransition {
    fn from(v: IdentityUpdateTransitionWasm) -> Self {
        v.0
    }
}

// pub fn js_value_to_identity_update_transition_object(object: JsValue) -> Result<Value, JsValue> {
//     let parameters: IdentityUpdateTransitionParams =
//         with_js_error!(serde_wasm_bindgen::from_value(object))?;
//
//     platform_value::to_value(parameters).map_err(|e| e.to_string().into())
// }

#[wasm_bindgen(js_class = IdentityUpdateTransition)]
impl IdentityUpdateTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(platform_version: u32) -> Result<IdentityUpdateTransitionWasm, JsValue> {
        let platform_version =
            &PlatformVersion::get(platform_version).map_err(|e| JsValue::from(e.to_string()))?;

        IdentityUpdateTransition::default_versioned(platform_version)
            .map(Into::into)
            .map_err(from_dpp_err)
    }

    #[wasm_bindgen(js_name=setPublicKeysToAdd)]
    pub fn set_public_keys_to_add(
        &mut self,
        public_keys: Option<Vec<JsValue>>,
    ) -> Result<(), JsValue> {
        let mut keys_to_add = vec![];
        if let Some(keys) = public_keys {
            keys_to_add = keys
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
        }

        self.0.set_public_keys_to_add(keys_to_add);

        Ok(())
    }

    #[wasm_bindgen(js_name=getPublicKeysToAdd)]
    pub fn get_public_keys_add(&self) -> Vec<JsValue> {
        self.0
            .public_keys_to_add()
            .iter()
            .map(|key| IdentityPublicKeyWithWitnessWasm::from(key.to_owned()).into())
            .collect()
    }

    #[wasm_bindgen(getter, js_name=addPublicKeys)]
    pub fn add_public_keys(&self) -> Vec<JsValue> {
        self.get_public_keys_add()
    }

    #[wasm_bindgen(js_name=getPublicKeyIdsToDisable)]
    pub fn get_public_key_ids_to_disable(&self) -> Vec<JsValue> {
        self.0
            .public_key_ids_to_disable()
            .iter()
            .map(|key| JsValue::from_f64(key.to_owned() as f64))
            .collect()
    }

    #[wasm_bindgen(js_name=setPublicKeyIdsToDisable)]
    pub fn set_public_key_ids_to_disable(&mut self, public_key_ids: Option<Vec<u32>>) {
        let mut keys = vec![];
        if let Some(public_key_ids) = public_key_ids {
            keys = public_key_ids
                .iter()
                .map(|key| key.to_owned() as KeyID)
                .collect::<Vec<KeyID>>();
        }

        self.0.set_public_key_ids_to_disable(keys);
    }

    #[wasm_bindgen(js_name=getType)]
    pub fn get_type(&self) -> u8 {
        self.0.state_transition_type() as u8
    }

    #[wasm_bindgen(getter, js_name=identityId)]
    pub fn identity_id(&self) -> IdentifierWrapper {
        self.get_identity_id()
    }

    #[wasm_bindgen(js_name=getIdentityId)]
    pub fn get_identity_id(&self) -> IdentifierWrapper {
        self.0.identity_id().to_owned().into()
    }

    #[wasm_bindgen(js_name=setIdentityId)]
    pub fn set_identity_id(&mut self, identity_id: &IdentifierWrapper) {
        self.0.set_identity_id(identity_id.to_owned().into());
    }

    #[wasm_bindgen(js_name=getOwnerId)]
    pub fn get_owner_id(&self) -> IdentifierWrapper {
        StateTransitionLike::owner_id(&self.0).to_owned().into()
    }

    #[wasm_bindgen(js_name=toObject)]
    pub fn to_object(&self, options: JsValue) -> Result<JsValue, JsValue> {
        let opts: super::to_object::ToObjectOptions = if options.is_object() {
            with_js_error!(serde_wasm_bindgen::from_value(options.clone()))?
        } else {
            Default::default()
        };

        let object = super::to_object::to_object_struct(&self.0, &opts);
        let js_object = js_sys::Object::new();

        js_sys::Reflect::set(
            &js_object,
            &"type".to_owned().into(),
            &object.transition_type.into(),
        )?;

        let version = match self.0 {
            IdentityUpdateTransition::V0(_) => "0",
        };

        js_sys::Reflect::set(&js_object, &"$version".to_owned().into(), &version.into())?;

        js_sys::Reflect::set(
            &js_object,
            &"revision".to_owned().into(),
            &object.revision.into(),
        )?;

        if let Some(signature) = object.signature {
            let signature_value: JsValue = if signature.is_empty() {
                JsValue::undefined()
            } else {
                Buffer::from_bytes(&signature).into()
            };

            js_sys::Reflect::set(&js_object, &"signature".to_owned().into(), &signature_value)?;

            js_sys::Reflect::set(
                &js_object,
                &"signaturePublicKeyId".to_owned().into(),
                &JsValue::from(object.signature_public_key_id),
            )?;
        }

        if let Some(public_keys_to_add) = object.public_keys_to_add {
            let keys_objects = public_keys_to_add
                .into_iter()
                .map(|key| {
                    IdentityPublicKeyWithWitnessWasm::from(key)
                        .to_object(opts.skip_signature.unwrap_or(false))
                })
                .collect::<Result<js_sys::Array, _>>()?;

            js_sys::Reflect::set(
                &js_object,
                &"addPublicKeys".to_owned().into(),
                &keys_objects,
            )?;
        }

        if let Some(public_key_ids_to_disable) = object.public_key_ids_to_disable {
            let public_key_ids_to_disable = public_key_ids_to_disable
                .into_iter()
                .map(|key| JsValue::from_f64(key as f64))
                .collect::<js_sys::Array>();

            js_sys::Reflect::set(
                &js_object,
                &"disablePublicKeys".to_owned().into(),
                &public_key_ids_to_disable.into(),
            )?;
        }

        js_sys::Reflect::set(
            &js_object,
            &"identityId".to_owned().into(),
            &Buffer::from_bytes(object.identity_id.to_buffer().as_slice()),
        )?;

        Ok(js_object.into())
    }

    #[wasm_bindgen(js_name=toBuffer)]
    pub fn to_buffer(&self) -> Result<Buffer, JsValue> {
        let bytes = PlatformSerializable::serialize_to_bytes(&StateTransition::IdentityUpdate(
            self.0.clone(),
        ))
        .with_js_error()?;
        Ok(Buffer::from_bytes(&bytes))
    }

    #[wasm_bindgen(js_name=toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        let object = super::to_object::to_object_struct(&self.0, &Default::default());
        let js_object = js_sys::Object::new();

        js_sys::Reflect::set(
            &js_object,
            &"type".to_owned().into(),
            &object.transition_type.into(),
        )?;

        let version = match self.0 {
            IdentityUpdateTransition::V0(_) => "0",
        };

        js_sys::Reflect::set(&js_object, &"$version".to_owned().into(), &version.into())?;

        js_sys::Reflect::set(
            &js_object,
            &"revision".to_owned().into(),
            &object.revision.into(),
        )?;

        if let Some(signature) = object.signature {
            let signature_value: JsValue = if signature.is_empty() {
                JsValue::undefined()
            } else {
                string_encoding::encode(signature.as_slice(), Encoding::Base64).into()
            };

            js_sys::Reflect::set(&js_object, &"signature".to_owned().into(), &signature_value)?;

            js_sys::Reflect::set(
                &js_object,
                &"signaturePublicKeyId".to_owned().into(),
                &object.signature_public_key_id.into(),
            )?;
        }

        if let Some(public_keys_to_add) = object.public_keys_to_add {
            let keys_objects = public_keys_to_add
                .into_iter()
                .map(|key| IdentityPublicKeyWithWitnessWasm::from(key).to_json())
                .collect::<Result<js_sys::Array, _>>()?;

            js_sys::Reflect::set(
                &js_object,
                &"addPublicKeys".to_owned().into(),
                &keys_objects,
            )?;
        }

        if let Some(public_key_ids_to_disable) = object.public_key_ids_to_disable {
            let public_key_ids_to_disable = public_key_ids_to_disable
                .into_iter()
                .map(|key| JsValue::from_f64(key as f64))
                .collect::<js_sys::Array>();

            js_sys::Reflect::set(
                &js_object,
                &"disablePublicKeys".to_owned().into(),
                &public_key_ids_to_disable.into(),
            )?;
        }

        let identity_id = object.identity_id.to_string(Encoding::Base58);

        js_sys::Reflect::set(
            &js_object,
            &"identityId".to_owned().into(),
            &identity_id.into(),
        )?;

        Ok(js_object.into())
    }

    #[wasm_bindgen(js_name=getModifiedDataIds)]
    pub fn modified_data_ids(&self) -> Vec<JsValue> {
        let ids = self.0.modified_data_ids();

        ids.into_iter()
            .map(|id| <IdentifierWrapper as std::convert::From<Identifier>>::from(id).into())
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

    #[wasm_bindgen(js_name=isVotingStateTransition)]
    pub fn is_voting_state_transition(&self) -> bool {
        self.0.is_voting_state_transition()
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

        // TODO: not the best approach because it involves cloning the transition
        // Probably it worth to return `sign_by_private_key` per state transition
        let mut wrapper = StateTransition::IdentityUpdate(self.0.clone());
        wrapper
            .sign_by_private_key(private_key.as_slice(), key_type, &bls_adapter)
            .with_js_error()?;

        self.0.set_signature(wrapper.signature().to_owned());

        Ok(())
    }

    #[wasm_bindgen(js_name=setSignaturePublicKeyId)]
    pub fn set_signature_public_key_id(&mut self, key_id: Option<u32>) {
        self.0.set_signature_public_key_id(key_id.unwrap_or(0))
    }

    #[wasm_bindgen(js_name=getSignature)]
    pub fn get_signature(&self) -> Buffer {
        Buffer::from_bytes_owned(self.0.signature().to_vec())
    }

    #[wasm_bindgen(js_name=setSignature)]
    pub fn set_signature(&mut self, signature: Option<Vec<u8>>) {
        self.0
            .set_signature(BinaryData::new(signature.unwrap_or_default()))
    }

    #[wasm_bindgen(js_name=getRevision)]
    pub fn get_revision(&self) -> u32 {
        self.0.revision() as u32
    }

    #[wasm_bindgen(js_name=setRevision)]
    pub fn set_revision(&mut self, revision: u32) {
        self.0.set_revision(revision as u64)
    }

    #[wasm_bindgen]
    pub fn sign(
        &mut self,
        identity_public_key: &IdentityPublicKeyWasm,
        private_key: Vec<u8>,
        bls: JsBlsAdapter,
    ) -> Result<(), JsValue> {
        let bls_adapter = BlsAdapter(bls);
        // TODO: come up with a better way to set signature to the binding.
        let mut state_transition = StateTransition::IdentityUpdate(self.0.clone());
        state_transition
            .sign(
                &identity_public_key.to_owned().into(),
                &private_key,
                &bls_adapter,
            )
            .with_js_error()?;

        let signature = state_transition.signature().to_owned();
        let signature_public_key_id = state_transition.signature_public_key_id().unwrap_or(0);

        self.0.set_signature(signature);
        self.0.set_signature_public_key_id(signature_public_key_id);

        Ok(())
    }

    #[wasm_bindgen(js_name=verifySignature)]
    pub fn verify_signature(
        &self,
        identity_public_key: &IdentityPublicKeyWasm,
        bls: JsBlsAdapter,
    ) -> Result<bool, JsValue> {
        let bls_adapter = BlsAdapter(bls);

        let verification_result = StateTransition::IdentityUpdate(self.0.clone())
            .verify_signature(&identity_public_key.to_owned().into(), &bls_adapter);

        match verification_result {
            Ok(()) => Ok(true),
            Err(protocol_error) => match &protocol_error {
                ProtocolError::ConsensusError(err) => match err.as_ref() {
                    ConsensusError::SignatureError(
                        SignatureError::InvalidStateTransitionSignatureError { .. },
                    ) => Ok(false),
                    _ => Err(protocol_error),
                },
                _ => Err(protocol_error),
            },
        }
        .with_js_error()
    }
}
