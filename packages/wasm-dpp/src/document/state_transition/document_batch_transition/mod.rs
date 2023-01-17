use dpp::{
    document::{
        state_transition::documents_batch_transition::property_names, DocumentsBatchTransition,
    },
    prelude::{DataContract, Document, Identifier},
    state_transition::{
        state_transition_execution_context::StateTransitionExecutionContext,
        StateTransitionConvert, StateTransitionIdentitySigned, StateTransitionLike,
        StateTransitionType,
    },
    util::json_value::JsonValueExt,
};
use js_sys::{Array, Reflect};
use serde::Serialize;
use wasm_bindgen::prelude::*;
use dpp::identity::KeyID;

use crate::{
    bls_adapter::{BlsAdapter, JsBlsAdapter},
    buffer::Buffer,
    document_batch_transition::document_transition::DocumentTransitionWasm,
    identifier::IdentifierWrapper,
    lodash::lodash_set,
    utils::{ToSerdeJSONExt, WithJsError},
    DocumentWasm, IdentityPublicKeyWasm,
};
pub mod document_transition;

#[derive(Debug)]
#[wasm_bindgen(js_name = DocumentsBatchTransition)]
pub struct DocumentsBatchTransitionWASM(DocumentsBatchTransition);

/// Collections of Documents split by actions
#[derive(Debug, Default)]
#[wasm_bindgen(js_name=DocumentsContainer)]
pub struct DocumentsContainer {
    create: Vec<Document>,
    replace: Vec<Document>,
    delete: Vec<Document>,
}

#[wasm_bindgen(js_class=DocumentsContainer)]
impl DocumentsContainer {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Default::default()
    }

    #[wasm_bindgen(js_name=pushDocumentCreate)]
    pub fn push_document_create(&mut self, d: DocumentWasm) {
        self.create.push(d.0);
    }

    #[wasm_bindgen(js_name=pushDocumentReplace)]
    pub fn push_document_replace(&mut self, d: DocumentWasm) {
        self.replace.push(d.0);
    }

    #[wasm_bindgen(js_name=pushDocumentDelete)]
    pub fn push_document_delete(&mut self, d: DocumentWasm) {
        self.delete.push(d.0);
    }
}

impl DocumentsContainer {
    pub fn take_documents_create(&mut self) -> Vec<Document> {
        std::mem::take(&mut self.create)
    }

    pub fn take_documents_replace(&mut self) -> Vec<Document> {
        std::mem::take(&mut self.replace)
    }

    pub fn take_documents_delete(&mut self) -> Vec<Document> {
        std::mem::take(&mut self.delete)
    }
}

#[wasm_bindgen(js_class=DocumentsBatchTransition)]
impl DocumentsBatchTransitionWASM {
    #[wasm_bindgen(constructor)]
    pub fn from_raw_object(
        js_raw_transition: JsValue,
        data_contracts: Array,
    ) -> Result<DocumentsBatchTransitionWASM, JsValue> {
        let data_contracts_array_js = Array::from(&data_contracts);

        let mut data_contracts: Vec<DataContract> = vec![];
        for contract in data_contracts_array_js.iter() {
            let json_value = contract.with_serde_to_json_value()?;
            let data_contract = DataContract::from_json_object(json_value).with_js_error()?;
            data_contracts.push(data_contract);
        }

        let documents_batch_transition = DocumentsBatchTransition::from_json_object(
            js_raw_transition.with_serde_to_json_value()?,
            data_contracts,
        )
        .with_js_error()?;

        Ok(documents_batch_transition.into())
    }

    #[wasm_bindgen(js_name=getType)]
    pub fn get_type(&self) -> u8 {
        StateTransitionType::DocumentsBatch.into()
    }

    #[wasm_bindgen(js_name=getOwnerId)]
    pub fn get_owner_id(&self) -> IdentifierWrapper {
        self.0.get_owner_id().to_owned().into()
    }

    #[wasm_bindgen(js_name=getTransitions)]
    pub fn get_transitions(&self) -> js_sys::Array {
        let array = js_sys::Array::new();
        let transitions = self.0.get_transitions();

        for tr in transitions.iter().cloned() {
            let transition: DocumentTransitionWasm = tr.into();
            array.push(&transition.into());
        }

        array
    }

    #[wasm_bindgen(js_name=toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        let value = self.0.to_json(false).with_js_error()?;
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();

        let is_null_signature = value.get(property_names::SIGNATURE).is_none();
        let is_null_signature_public_key_id =
            value.get(property_names::SIGNATURE_PUBLIC_KEY_ID).is_none();

        let js_value = value.serialize(&serializer)?;

        if is_null_signature {
            js_sys::Reflect::set(
                &js_value,
                &property_names::SIGNATURE.into(),
                &JsValue::undefined(),
            )?;
        }
        if is_null_signature_public_key_id {
            js_sys::Reflect::set(
                &js_value,
                &property_names::SIGNATURE_PUBLIC_KEY_ID.into(),
                &JsValue::undefined(),
            )?;
        }

        Ok(js_value)
    }

    #[wasm_bindgen(js_name=toObject)]
    pub fn to_object(&self, options: &JsValue) -> Result<JsValue, JsValue> {
        let skip_signature = if options.is_object() {
            let options = options.with_serde_to_json_value()?;
            options.get_bool("skipSignature").unwrap_or_default()
        } else {
            false
        };

        let mut value = self.0.to_object(skip_signature).with_js_error()?;
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        let js_value = value.serialize(&serializer)?;

        // Transform every transition individually
        let transitions = Array::new();
        for transition in self.0.transitions.iter() {
            let js_value =
                DocumentTransitionWasm::from(transition.to_owned()).to_object(options)?;
            transitions.push(&js_value);
        }
        // Replace the whole collection of transitions
        Reflect::set(
            &js_value,
            &property_names::TRANSITIONS.into(),
            &transitions.into(),
        )?;

        // Transform paths that are specific to the DocumentsBatchTransition
        for path in DocumentsBatchTransition::binary_property_paths() {
            if let Ok(bytes) = value.remove_path_into::<Vec<u8>>(path) {
                let buffer = Buffer::from_bytes(&bytes);
                lodash_set(&js_value, path, buffer.into());
            }
        }
        for path in DocumentsBatchTransition::identifiers_property_paths() {
            if let Ok(bytes) = value.remove_path_into::<Vec<u8>>(path) {
                let id = IdentifierWrapper::new(bytes)?;
                lodash_set(&js_value, path, id.into());
            }
        }

        if value.get(property_names::SIGNATURE).is_none() && !skip_signature {
            js_sys::Reflect::set(
                &js_value,
                &property_names::SIGNATURE.into(),
                &JsValue::undefined(),
            )?;
        }
        if value.get(property_names::SIGNATURE_PUBLIC_KEY_ID).is_none() {
            js_sys::Reflect::set(
                &js_value,
                &property_names::SIGNATURE_PUBLIC_KEY_ID.into(),
                &JsValue::undefined(),
            )?;
        }

        Ok(js_value)
    }

    #[wasm_bindgen(js_name=getModifiedDataIds)]
    pub fn get_modified_ids(&self) -> Array {
        let array = Array::new();

        for id in self.0.get_modified_data_ids() {
            let id = <IdentifierWrapper as From<Identifier>>::from(id.to_owned());
            array.push(&id.into());
        }

        array
    }

    // AbstractSTateTransitionIdentitySigned methods
    #[wasm_bindgen(js_name=getSignaturePublicKeyId)]
    pub fn get_signature_public_key_id(&self) -> Option<f64> {
        self.0.get_signature_public_key_id().map(|v| v as f64)
    }

    #[wasm_bindgen(js_name=sign)]
    pub fn sign(
        &mut self,
        identity_public_key: &IdentityPublicKeyWasm,
        private_key: &[u8],
        bls: JsBlsAdapter,
    ) -> Result<(), JsValue> {
        self.0
            .sign(identity_public_key.inner(), private_key, &BlsAdapter(bls))
            .with_js_error()
    }

    #[wasm_bindgen(js_name=verifyPublicKeyLevelAndPurpose)]
    pub fn verify_public_key_level_and_purpose(
        &self,
        public_key: &IdentityPublicKeyWasm,
    ) -> Result<(), JsValue> {
        self.0
            .verify_public_key_level_and_purpose(public_key.inner())
            .with_js_error()
    }

    #[wasm_bindgen(js_name=verifyPublicKeyIsEnabled)]
    pub fn verify_public_key_is_enabled(
        &self,
        public_key: &IdentityPublicKeyWasm,
    ) -> Result<(), JsValue> {
        self.0
            .verify_public_key_is_enabled(public_key.inner())
            .with_js_error()
    }

    #[wasm_bindgen(js_name=verifySignature)]
    pub fn verify_signature(
        &self,
        public_key: &IdentityPublicKeyWasm,
        bls: JsBlsAdapter,
    ) -> Result<(), JsValue> {
        self.0
            .verify_signature(public_key.inner(), &BlsAdapter(bls))
            .with_js_error()
    }

    #[wasm_bindgen(js_name=setSignaturePublicKey)]
    pub fn set_signature_public_key(&mut self, key_id: KeyID) {
        self.0.set_signature_public_key_id(key_id)
    }

    #[wasm_bindgen(js_name=getSecurityLevelRequirement)]
    pub fn get_security_level_requirement(&self) -> u8 {
        self.0.get_security_level_requirement() as u8
    }

    // AbstractStateTransition methods
    #[wasm_bindgen(js_name=getProtocolVersion)]
    pub fn get_protocol_version(&self) -> u32 {
        self.0.get_protocol_version()
    }

    #[wasm_bindgen(js_name=getSignature)]
    pub fn get_signature(&self) -> Vec<u8> {
        self.0.get_signature().to_owned()
    }

    #[wasm_bindgen(js_name=setSignature)]
    pub fn set_signature(&mut self, signature: Vec<u8>) {
        self.0.set_signature(signature)
    }

    #[wasm_bindgen(js_name=calculateFee)]
    pub fn calculate_fee(&self) -> i64 {
        self.0.calculate_fee()
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

    #[wasm_bindgen(js_name=setExecutionContext)]
    pub fn set_execution_context(&mut self, context: StateExecutionContext) {
        self.0.set_execution_context(context.into_inner());
    }

    #[wasm_bindgen(js_name=getExecutionContext)]
    pub fn get_execution_context(&mut self) -> StateExecutionContext {
        StateExecutionContext(self.0.get_execution_context().to_owned())
    }

    #[wasm_bindgen(js_name=toBuffer)]
    pub fn to_buffer(&self, options: &JsValue) -> Result<Buffer, JsValue> {
        let skip_signature = if options.is_object() {
            let options = options.with_serde_to_json_value()?;
            options.get_bool("skipSignature").unwrap_or_default()
        } else {
            false
        };
        let bytes = self.0.to_buffer(skip_signature).with_js_error()?;

        Ok(Buffer::from_bytes(&bytes))
    }

    #[wasm_bindgen(js_name=hash)]
    pub fn hash(&self, options: JsValue) -> Result<Buffer, JsValue> {
        let skip_signature = if options.is_object() {
            let options = options.with_serde_to_json_value()?;
            options.get_bool("skipSignature").unwrap_or_default()
        } else {
            false
        };
        let bytes = self.0.hash(skip_signature).with_js_error()?;

        Ok(Buffer::from_bytes(&bytes))
    }
}

#[wasm_bindgen(js_name=StateExecutionContext)]
pub struct StateExecutionContext(StateTransitionExecutionContext);

impl StateExecutionContext {
    pub fn into_inner(self) -> StateTransitionExecutionContext {
        self.0
    }

    pub fn inner(&self) -> &StateTransitionExecutionContext {
        &self.0
    }

    pub fn inner_mut(&mut self) -> &mut StateTransitionExecutionContext {
        &mut self.0
    }
}

impl From<DocumentsBatchTransition> for DocumentsBatchTransitionWASM {
    fn from(t: DocumentsBatchTransition) -> Self {
        DocumentsBatchTransitionWASM(t)
    }
}
