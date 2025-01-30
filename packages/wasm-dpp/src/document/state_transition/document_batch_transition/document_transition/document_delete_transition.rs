use dpp::state_transition::documents_batch_transition::{
    document_delete_transition, DocumentDeleteTransition,
};

use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::{
    data_contract::DataContractWasm,
    document::document_batch_transition::document_transition::to_object,
    identifier::IdentifierWrapper, utils::WithJsError,
};
use dpp::state_transition::documents_batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use dpp::state_transition::documents_batch_transition::document_delete_transition::v0::v0_methods::DocumentDeleteTransitionV0Methods;
use dpp::state_transition::documents_batch_transition::document_transition::action_type::DocumentTransitionActionType;

#[wasm_bindgen(js_name=DocumentDeleteTransition)]
#[derive(Debug, Clone)]
pub struct DocumentDeleteTransitionWasm {
    inner: DocumentDeleteTransition,
}

impl From<DocumentDeleteTransition> for DocumentDeleteTransitionWasm {
    fn from(v: DocumentDeleteTransition) -> Self {
        Self { inner: v }
    }
}

impl From<DocumentDeleteTransitionWasm> for DocumentDeleteTransition {
    fn from(v: DocumentDeleteTransitionWasm) -> Self {
        v.inner
    }
}

#[wasm_bindgen(js_class=DocumentDeleteTransition)]
impl DocumentDeleteTransitionWasm {
    #[wasm_bindgen(js_name=getAction)]
    pub fn action(&self) -> u8 {
        DocumentTransitionActionType::Delete as u8
    }

    #[wasm_bindgen(js_name=toObject)]
    pub fn to_object(&self, options: &JsValue) -> Result<JsValue, JsValue> {
        to_object(
            self.inner.to_object().with_js_error()?,
            options,
            document_delete_transition::v0::IDENTIFIER_FIELDS,
            [],
        )
    }

    #[wasm_bindgen(js_name=toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        let value = self.inner.to_json().with_js_error()?;
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        let js_value = value.serialize(&serializer)?;
        Ok(js_value)
    }

    // AbstractDocumentTransition
    #[wasm_bindgen(js_name=getId)]
    pub fn id(&self) -> IdentifierWrapper {
        self.inner.base().id().into()
    }

    #[wasm_bindgen(js_name=getType)]
    pub fn document_type(&self) -> String {
        self.inner.base().document_type_name().clone()
    }

    #[wasm_bindgen(js_name = getEntropy)]
    pub fn get_entropy(&self) -> Vec<u8> {
        Vec::from(self.inner.entropy())
    }

    #[wasm_bindgen(js_name=getDataContractId)]
    pub fn data_contract_id(&self) -> IdentifierWrapper {
        self.inner.base().data_contract_id().into()
    }

    #[wasm_bindgen(js_name=getIdentityContractNonce)]
    pub fn get_identity_contract_nonce(&self) -> u64 {
        self.inner.base().identity_contract_nonce() as u64
    }

    #[wasm_bindgen(js_name=setIdentityContractNonce)]
    pub fn set_identity_contract_nonce(&mut self, identity_contract_nonce: u64) -> () {
        let mut base = self.inner.base().clone();

        base.set_identity_contract_nonce(identity_contract_nonce);

        self.inner.set_base(base)
    }

    #[wasm_bindgen(js_name=get)]
    pub fn get(&self, path: String) -> Result<JsValue, JsValue> {
        let _ = path;
        Ok(JsValue::undefined())
    }
}
