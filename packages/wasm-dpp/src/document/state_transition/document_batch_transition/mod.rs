use dpp::identity::{KeyID, Purpose};

use dpp::{
    prelude::Identifier,
    state_transition::{StateTransitionLike, StateTransitionType},
    ProtocolError,
};
use js_sys::Array;
use serde::{Deserialize, Serialize};

use dpp::consensus::signature::SignatureError;
use dpp::consensus::ConsensusError;
use dpp::platform_value::BinaryData;
use dpp::serialization::PlatformSerializable;
use dpp::state_transition::documents_batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::documents_batch_transition::document_transition::DocumentTransition;
use dpp::state_transition::documents_batch_transition::DocumentsBatchTransition;
use dpp::state_transition::StateTransition;
use wasm_bindgen::prelude::*;

use crate::{
    bls_adapter::{BlsAdapter, JsBlsAdapter},
    buffer::Buffer,
    identifier::IdentifierWrapper,
    utils::{IntoWasm, WithJsError},
    IdentityPublicKeyWasm,
};

use document_transition::DocumentTransitionWasm;
use dpp::state_transition::documents_batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;

use dpp::state_transition::StateTransitionIdentitySigned;

pub mod document_transition;
// pub mod validation;

#[derive(Clone, Debug)]
#[wasm_bindgen(js_name = DocumentsBatchTransition)]
pub struct DocumentsBatchTransitionWasm(DocumentsBatchTransition);

#[derive(Debug, Serialize, Deserialize, Default, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct ToObjectOptions {
    #[serde(default)]
    skip_signature: bool,
    #[serde(default)]
    skip_identifiers_conversion: bool,
}

#[wasm_bindgen(js_class=DocumentsBatchTransition)]
impl DocumentsBatchTransitionWasm {
    // #[wasm_bindgen(constructor)]
    // pub fn from_object(
    //     js_raw_transition: JsValue,
    //     data_contracts: Array,
    // ) -> Result<DocumentsBatchTransitionWasm, JsValue> {
    //     let data_contracts_array_js = Array::from(&data_contracts);
    //
    //     let mut data_contracts: Vec<DataContract> = vec![];
    //
    //     for contract in data_contracts_array_js.iter() {
    //         let value = contract.with_serde_to_platform_value()?;
    //         let data_contract = DataContract::from_value(value).with_js_error()?;
    //         data_contracts.push(data_contract);
    //     }
    //
    //     let mut batch_transition_value = js_raw_transition.with_serde_to_platform_value()?;
    //     let base_identifier_fields = document_base_transition::IDENTIFIER_FIELDS
    //         .iter()
    //         .map(|field| format!("{}[].{}", property_names::TRANSITIONS, field))
    //         .collect::<Vec<_>>();
    //     batch_transition_value
    //         .replace_at_paths(
    //             DocumentsBatchTransition::identifiers_property_paths()
    //                 .into_iter()
    //                 .chain(base_identifier_fields.iter().map(|s| s.as_str())),
    //             ReplacementType::Identifier,
    //         )
    //         .map_err(ProtocolError::ValueError)
    //         .with_js_error()?;
    //
    //     let documents_batch_transition = DocumentsBatchTransition::from_object_with_contracts(
    //         batch_transition_value,
    //         data_contracts,
    //     )
    //     .with_js_error()?;
    //
    //     Ok(documents_batch_transition.into())
    // }

    #[wasm_bindgen(js_name=getType)]
    pub fn get_type(&self) -> u8 {
        StateTransitionType::DocumentsBatch.into()
    }

    #[wasm_bindgen(js_name=getOwnerId)]
    pub fn get_owner_id(&self) -> IdentifierWrapper {
        self.0.owner_id().to_owned().into()
    }

    #[wasm_bindgen(js_name=getTransitions)]
    pub fn get_transitions(&self) -> js_sys::Array {
        let array = js_sys::Array::new();
        let transitions = self.0.transitions();

        for tr in transitions.iter().cloned() {
            let transition: DocumentTransitionWasm = tr.into();
            array.push(&transition.into());
        }

        array
    }

    #[wasm_bindgen(js_name=setTransitions)]
    pub fn set_transitions(&mut self, js_transitions: Array) -> Result<(), JsValue> {
        let mut transitions = vec![];
        for js_transition in js_transitions.iter() {
            let transition: DocumentTransition = js_transition
                .to_wasm::<DocumentTransitionWasm>("DocumentTransition")?
                .to_owned()
                .into();
            transitions.push(transition)
        }

        self.0.set_transitions(transitions);

        Ok(())
    }

    #[wasm_bindgen(js_name=setIdentityContractNonce)]
    pub fn set_identity_contract_nonce(&mut self, nonce: u32) {
        self.0.set_identity_contract_nonce(nonce as u64);
    }

    // #[wasm_bindgen(js_name=toJSON)]
    // pub fn to_json(&self) -> Result<JsValue, JsValue> {
    //     let value = self.0.to_json(false).with_js_error()?;
    //     let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    //
    //     let is_null_signature = value.get(property_names::SIGNATURE).is_none();
    //     let is_null_signature_public_key_id =
    //         value.get(property_names::SIGNATURE_PUBLIC_KEY_ID).is_none();
    //
    //     let js_value = value.serialize(&serializer)?;
    //
    //     if is_null_signature {
    //         js_sys::Reflect::set(
    //             &js_value,
    //             &property_names::SIGNATURE.into(),
    //             &JsValue::undefined(),
    //         )?;
    //     }
    //     if is_null_signature_public_key_id {
    //         js_sys::Reflect::set(
    //             &js_value,
    //             &property_names::SIGNATURE_PUBLIC_KEY_ID.into(),
    //             &JsValue::undefined(),
    //         )?;
    //     }
    //
    //     Ok(js_value)
    // }

    // #[wasm_bindgen(js_name=toObject)]
    // pub fn to_object(&self, js_options: &JsValue) -> Result<JsValue, JsValue> {
    //     let options: ToObjectOptions = if js_options.is_object() {
    //         let raw_options = js_options.with_serde_to_json_value()?;
    //         serde_json::from_value(raw_options).with_js_error()?
    //     } else {
    //         Default::default()
    //     };
    //
    //     let mut value = self
    //         .0
    //         .to_cleaned_object(options.skip_signature)
    //         .with_js_error()?;
    //     let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    //     let js_value = value.serialize(&serializer)?;
    //     let is_signature_present = value
    //         .get(property_names::SIGNATURE)
    //         .map_err(ProtocolError::ValueError)
    //         .with_js_error()?
    //         .is_some();
    //
    //     // Transform every transition individually
    //     let transitions = Array::new();
    //     for transition in self.0.transitions.iter() {
    //         let js_value =
    //             DocumentTransitionWasm::from(transition.to_owned()).to_object(js_options)?;
    //         transitions.push(&js_value);
    //     }
    //     // Replace the whole collection of transitions
    //     Reflect::set(
    //         &js_value,
    //         &property_names::TRANSITIONS.into(),
    //         &transitions.into(),
    //     )?;
    //
    //     // Transform paths that are specific to the DocumentsBatchTransition
    //     for path in DocumentsBatchTransition::binary_property_paths() {
    //         if let Some(bytes) = value
    //             .remove_optional_value_at_path(path)
    //             .and_then(|value| value.map(|value| value.to_binary_bytes()).transpose())
    //             .map_err(ProtocolError::ValueError)
    //             .with_js_error()?
    //         {
    //             let buffer = Buffer::from_bytes_owned(bytes);
    //             lodash_set(&js_value, path, buffer.into());
    //         }
    //     }
    //     for path in DocumentsBatchTransition::identifiers_property_paths() {
    //         if let Some(bytes) = value
    //             .remove_optional_value_at_path(path)
    //             .and_then(|value| value.map(|value| value.to_identifier_bytes()).transpose())
    //             .map_err(ProtocolError::ValueError)
    //             .with_js_error()?
    //         {
    //             let buffer = Buffer::from_bytes_owned(bytes);
    //             if !options.skip_identifiers_conversion {
    //                 lodash_set(&js_value, path, buffer.into());
    //             } else {
    //                 let id = IdentifierWrapper::new(buffer.into());
    //                 lodash_set(&js_value, path, id.into());
    //             }
    //         }
    //     }
    //
    //     if !is_signature_present && !options.skip_signature {
    //         js_sys::Reflect::set(
    //             &js_value,
    //             &property_names::SIGNATURE.into(),
    //             &JsValue::undefined(),
    //         )?;
    //     }
    //     if value
    //         .get(property_names::SIGNATURE_PUBLIC_KEY_ID)
    //         .map_err(ProtocolError::ValueError)
    //         .with_js_error()?
    //         .is_none()
    //     {
    //         js_sys::Reflect::set(
    //             &js_value,
    //             &property_names::SIGNATURE_PUBLIC_KEY_ID.into(),
    //             &JsValue::undefined(),
    //         )?;
    //     }
    //
    //     Ok(js_value)
    // }

    #[wasm_bindgen(js_name=getModifiedDataIds)]
    pub fn get_modified_ids(&self) -> Array {
        let array = Array::new();

        for id in self.0.modified_data_ids() {
            let id = <IdentifierWrapper as From<Identifier>>::from(id.to_owned());
            array.push(&id.into());
        }

        array
    }

    // AbstractSTateTransitionIdentitySigned methods
    #[wasm_bindgen(js_name=getSignaturePublicKeyId)]
    pub fn get_signature_public_key_id(&self) -> u32 {
        self.0.signature_public_key_id()
    }

    #[wasm_bindgen(js_name=sign)]
    pub fn sign(
        &mut self,
        identity_public_key: &IdentityPublicKeyWasm,
        private_key: &[u8],
        bls: JsBlsAdapter,
    ) -> Result<(), JsValue> {
        let bls_adapter = BlsAdapter(bls);

        // TODO: come up with a better way to set signature to the binding.
        let mut state_transition = StateTransition::DocumentsBatch(self.0.clone());
        state_transition
            .sign(
                &identity_public_key.to_owned().into(),
                private_key,
                &bls_adapter,
            )
            .with_js_error()?;

        let signature = state_transition.signature().to_owned();
        let signature_public_key_id = state_transition.signature_public_key_id().unwrap_or(0);

        self.0.set_signature(signature);
        self.0.set_signature_public_key_id(signature_public_key_id);

        Ok(())
    }
    //
    // #[wasm_bindgen(js_name=verifyPublicKeyLevelAndPurpose)]
    // pub fn verify_public_key_level_and_purpose(
    //     &self,
    //     public_key: &IdentityPublicKeyWasm,
    // ) -> Result<(), JsValue> {
    //     self.0
    //         .verify_public_key_level_and_purpose(public_key.inner())
    //         .with_js_error()
    // }
    //
    // #[wasm_bindgen(js_name=verifyPublicKeyIsEnabled)]
    // pub fn verify_public_key_is_enabled(
    //     &self,
    //     public_key: &IdentityPublicKeyWasm,
    // ) -> Result<(), JsValue> {
    //     self.0
    //         .verify_public_key_is_enabled(public_key.inner())
    //         .with_js_error()
    // }

    #[wasm_bindgen(js_name=verifySignature)]
    pub fn verify_signature(
        &self,
        identity_public_key: &IdentityPublicKeyWasm,
        bls: JsBlsAdapter,
    ) -> Result<bool, JsValue> {
        let bls_adapter = BlsAdapter(bls);

        let verification_result = StateTransition::DocumentsBatch(self.0.clone())
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

    #[wasm_bindgen(js_name=setSignaturePublicKeyId)]
    pub fn set_signature_public_key(&mut self, key_id: KeyID) {
        self.0.set_signature_public_key_id(key_id)
    }

    #[wasm_bindgen(js_name=getKeySecurityLevelRequirement)]
    pub fn get_security_level_requirement(&self, purpose: u8) -> Result<js_sys::Array, JsValue> {
        // Convert the integer to a Purpose enum
        let purpose_enum = match Purpose::try_from(purpose) {
            Ok(purpose) => purpose,
            Err(_) => {
                return Err(JsValue::from_str(
                    "Invalid purpose value, expected a number between 0 and 5.",
                ))
            }
        };

        let array = js_sys::Array::new();
        for security_level in self.0.security_level_requirement(purpose_enum) {
            array.push(&JsValue::from(security_level as u32));
        }
        Ok(array)
    }

    // AbstractStateTransition methods
    // #[wasm_bindgen(js_name=getProtocolVersion)]
    // pub fn get_protocol_version(&self) -> u32 {
    //     self.0.state_transition_protocol_version()
    // }

    #[wasm_bindgen(js_name=getSignature)]
    pub fn get_signature(&self) -> Vec<u8> {
        self.0.signature().to_vec()
    }

    #[wasm_bindgen(js_name=setSignature)]
    pub fn set_signature(&mut self, signature: Vec<u8>) {
        self.0.set_signature(BinaryData::new(signature))
    }

    #[wasm_bindgen(js_name=isDocumentStateTransition)]
    pub fn is_document_state_transition(&self) -> bool {
        self.0.is_document_state_transition()
    }

    #[wasm_bindgen(js_name=isDataContractStateTransition)]
    pub fn is_data_contract_state_transition(&self) -> bool {
        self.0.is_data_contract_state_transition()
    }

    #[wasm_bindgen(js_name=isIdentityStateTransition)]
    pub fn is_identity_state_transition(&self) -> bool {
        self.0.is_identity_state_transition()
    }

    #[wasm_bindgen(js_name=isVotingStateTransition)]
    pub fn is_voting_state_transition(&self) -> bool {
        self.0.is_voting_state_transition()
    }

    #[wasm_bindgen(js_name=toBuffer)]
    pub fn to_buffer(&self) -> Result<Buffer, JsValue> {
        let bytes = PlatformSerializable::serialize_to_bytes(&StateTransition::DocumentsBatch(
            self.0.clone(),
        ))
        .with_js_error()?;
        Ok(Buffer::from_bytes(&bytes))
    }

    // #[wasm_bindgen(js_name=hash)]
    // pub fn hash(&self, options: JsValue) -> Result<Buffer, JsValue> {
    //     let skip_signature = if options.is_object() {
    //         let options = options.with_serde_to_json_value()?;
    //         options.get_bool("skipSignature").unwrap_or_default()
    //     } else {
    //         false
    //     };
    //     let bytes = self.0.hash(skip_signature).with_js_error()?;
    //
    //     Ok(Buffer::from_bytes(&bytes))
    // }
}

impl From<DocumentsBatchTransition> for DocumentsBatchTransitionWasm {
    fn from(t: DocumentsBatchTransition) -> Self {
        DocumentsBatchTransitionWasm(t)
    }
}

impl From<DocumentsBatchTransitionWasm> for DocumentsBatchTransition {
    fn from(t: DocumentsBatchTransitionWasm) -> Self {
        t.0
    }
}
