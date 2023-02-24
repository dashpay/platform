use dpp::document::document_transition::{
    document_delete_transition, DocumentDeleteTransition, DocumentTransitionObjectLike,
};
use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::{
    document_batch_transition::document_transition::to_object, identifier::IdentifierWrapper,
    utils::WithJsError, DataContractWasm,
};

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

#[wasm_bindgen(js_class=DocumentDeleteTransition)]
impl DocumentDeleteTransitionWasm {
    #[wasm_bindgen(js_name=getAction)]
    pub fn action(&self) -> u8 {
        self.inner.base.action as u8
    }

    #[wasm_bindgen(js_name=toObject)]
    pub fn to_object(&self, options: &JsValue) -> Result<JsValue, JsValue> {
        to_object(
            &self.inner,
            options,
            document_delete_transition::IDENTIFIER_FIELDS,
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
        self.inner.base.id.clone().into()
    }

    #[wasm_bindgen(js_name=getType)]
    pub fn document_type(&self) -> String {
        self.inner.base.document_type.clone()
    }

    #[wasm_bindgen(js_name=getDataContract)]
    pub fn data_contract(&self) -> DataContractWasm {
        self.inner.base.data_contract.clone().into()
    }

    #[wasm_bindgen(js_name=getDataContractId)]
    pub fn data_contract_id(&self) -> IdentifierWrapper {
        self.inner.base.data_contract.id.clone().into()
    }

    #[wasm_bindgen(js_name=get)]
    pub fn get(&self, path: String) -> Result<JsValue, JsValue> {
        let _ = path;
        Ok(JsValue::undefined())
    }
}
