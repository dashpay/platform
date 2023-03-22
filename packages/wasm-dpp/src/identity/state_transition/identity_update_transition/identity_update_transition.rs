use std::convert::TryInto;
use std::default::Default;

use serde::{Deserialize, Serialize};
use wasm_bindgen::__rt::Ref;
use wasm_bindgen::prelude::*;

use crate::identifier::IdentifierWrapper;

use crate::{
    buffer::Buffer, errors::RustConversionError,
    identity::state_transition::identity_public_key_transitions::IdentityPublicKeyCreateTransitionWasm,
    identity::IdentityPublicKeyWasm, state_transition::StateTransitionExecutionContextWasm,
    with_js_error,
};

use crate::bls_adapter::{BlsAdapter, JsBlsAdapter};

use crate::utils::{generic_of_js_val, WithJsError};
use dpp::identity::state_transition::identity_public_key_transitions::IdentityPublicKeyWithWitness;
use dpp::identity::{KeyID, TimestampMillis};
use dpp::platform_value::string_encoding::Encoding;
use dpp::platform_value::{string_encoding, Value};
use dpp::prelude::Revision;
use dpp::state_transition::StateTransitionIdentitySigned;
use dpp::{
    identifier::Identifier,
    identity::state_transition::identity_update_transition::identity_update_transition::IdentityUpdateTransition,
    platform_value, state_transition::StateTransitionLike, ProtocolError,
};

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
    add_public_keys: Option<Vec<IdentityPublicKeyWithWitness>>,
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

pub fn js_value_to_identity_update_transition_object(object: JsValue) -> Result<Value, JsValue> {
    let parameters: IdentityUpdateTransitionParams =
        with_js_error!(serde_wasm_bindgen::from_value(object))?;

    platform_value::to_value(parameters).map_err(|e| e.to_string().into())
}

#[wasm_bindgen(js_class = IdentityUpdateTransition)]
impl IdentityUpdateTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(raw_parameters: JsValue) -> Result<IdentityUpdateTransitionWasm, JsValue> {
        let mut identity_update_transition_object =
            js_value_to_identity_update_transition_object(raw_parameters)?;

        IdentityUpdateTransition::clean_value(&mut identity_update_transition_object)
            .map_err(ProtocolError::ValueError)
            .with_js_error()?;

        let identity_update_transition =
            IdentityUpdateTransition::new(identity_update_transition_object)
                .map_err(|e| RustConversionError::Error(e.to_string()).to_js_value())?;

        Ok(identity_update_transition.into())
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
                    let public_key: Ref<IdentityPublicKeyCreateTransitionWasm> =
                        generic_of_js_val::<IdentityPublicKeyCreateTransitionWasm>(
                            value,
                            "IdentityPublicKeyCreateTransition",
                        )?;
                    Ok(public_key.clone().into())
                })
                .collect::<Result<Vec<IdentityPublicKeyWithWitness>, JsValue>>()?;
        }

        self.0.set_public_keys_to_add(keys_to_add);

        Ok(())
    }

    #[wasm_bindgen(js_name=getPublicKeysToAdd)]
    pub fn get_public_keys_add(&self) -> Vec<JsValue> {
        self.0
            .get_public_keys_to_add()
            .iter()
            .map(|key| IdentityPublicKeyCreateTransitionWasm::from(key.to_owned()).into())
            .collect()
    }

    #[wasm_bindgen(getter, js_name=addPublicKeys)]
    pub fn add_public_keys(&self) -> Vec<JsValue> {
        self.get_public_keys_add()
    }

    #[wasm_bindgen(js_name=getPublicKeyIdsToDisable)]
    pub fn get_public_key_ids_to_disable(&self) -> Vec<JsValue> {
        self.0
            .get_public_key_ids_to_disable()
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

    #[wasm_bindgen(js_name=getPublicKeysDisabledAt)]
    pub fn get_public_keys_disabled_at(&self) -> Option<js_sys::Date> {
        self.0
            .get_public_keys_disabled_at()
            .map(|timestamp| js_sys::Date::new(&JsValue::from_f64(timestamp as f64)))
    }

    #[wasm_bindgen(js_name=setPublicKeysDisabledAt)]
    pub fn set_public_keys_disabled_at(&mut self, timestamp: Option<js_sys::Date>) {
        if let Some(timestamp) = timestamp {
            self.0
                .set_public_keys_disabled_at(Some(timestamp.get_time() as TimestampMillis));
        } else {
            self.0.set_public_keys_disabled_at(None);
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
        (*self.0.get_identity_id()).into()
    }

    #[wasm_bindgen(js_name=setIdentityId)]
    pub fn set_identity_id(&mut self, identity_id: &IdentifierWrapper) {
        self.0.set_identity_id(identity_id.to_owned().into());
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

            if let Some(signature_public_key_id) = object.signature_public_key_id {
                js_sys::Reflect::set(
                    &js_object,
                    &"signaturePublicKeyId".to_owned().into(),
                    &JsValue::from(signature_public_key_id),
                )?;
            } else {
                js_sys::Reflect::set(
                    &js_object,
                    &"signaturePublicKeyId".to_owned().into(),
                    &JsValue::undefined(),
                )?;
            }
        }

        if let Some(timestamp) = object.public_keys_disabled_at {
            js_sys::Reflect::set(
                &js_object,
                &"publicKeysDisabledAt".to_owned().into(),
                &(timestamp as u32).into(),
            )?;
        }

        if let Some(public_keys_to_add) = object.public_keys_to_add {
            let keys_objects = public_keys_to_add
                .into_iter()
                .map(|key| {
                    IdentityPublicKeyCreateTransitionWasm::from(key).to_object(options.clone())
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

            if let Some(signature_public_key_id) = object.signature_public_key_id {
                js_sys::Reflect::set(
                    &js_object,
                    &"signaturePublicKeyId".to_owned().into(),
                    &signature_public_key_id.into(),
                )?;
            } else {
                js_sys::Reflect::set(
                    &js_object,
                    &"signaturePublicKeyId".to_owned().into(),
                    &JsValue::undefined(),
                )?;
            }
        }

        if let Some(timestamp) = object.public_keys_disabled_at {
            js_sys::Reflect::set(
                &js_object,
                &"publicKeysDisabledAt".to_owned().into(),
                &(timestamp as u32).into(),
            )?;
        }

        if let Some(public_keys_to_add) = object.public_keys_to_add {
            let keys_objects = public_keys_to_add
                .into_iter()
                .map(|key| IdentityPublicKeyCreateTransitionWasm::from(key).to_json())
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
    pub fn get_modified_data_ids(&self) -> Vec<JsValue> {
        let ids = self.0.get_modified_data_ids();

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
            .with_js_error()
    }

    #[wasm_bindgen(js_name=getSignature)]
    pub fn get_signature(&self) -> Buffer {
        Buffer::from_bytes_owned(self.0.get_signature().to_vec())
    }

    #[wasm_bindgen(js_name=getRevision)]
    pub fn get_revision(&self) -> u32 {
        self.0.get_revision() as u32
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
        self.0
            .sign(
                &identity_public_key.to_owned().into(),
                &private_key,
                &bls_adapter,
            )
            .with_js_error()
    }
}
