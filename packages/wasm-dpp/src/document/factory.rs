use std::sync::Arc;

use dpp::document::{
    document_factory::{DocumentFactory, FactoryOptions},
    document_transition::Action,
    fetch_and_validate_data_contract::DataContractFetcherAndValidator,
};
use wasm_bindgen::prelude::*;

use crate::{
    identifier::identifier_from_js_value,
    state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper},
    utils::{ToSerdeJSONExt, WithJsError},
    DataContractWasm, DocumentWasm, DocumentsBatchTransitionWASM, DocumentsContainer,
};

use super::validator::DocumentValidatorWasm;

#[wasm_bindgen(js_name=DocumentTransitions)]
#[derive(Debug, Default)]
pub struct DocumentTransitions {
    create: Vec<DocumentWasm>,
    replace: Vec<DocumentWasm>,
    delete: Vec<DocumentWasm>,
}

#[wasm_bindgen(js_class=DocumentTransitions)]
impl DocumentTransitions {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Default::default()
    }

    #[wasm_bindgen(js_name = "addTransitionCreate")]
    pub fn add_transition_create(&mut self, transition: DocumentWasm) {
        self.create.push(transition)
    }

    #[wasm_bindgen(js_name = "addTransitionReplace")]
    pub fn add_transition_replace(&mut self, transition: DocumentWasm) {
        self.replace.push(transition)
    }

    #[wasm_bindgen(js_name = "addTransitionDelete")]
    pub fn add_transition_delete(&mut self, transition: DocumentWasm) {
        self.delete.push(transition)
    }
}

#[wasm_bindgen(js_name = DocumentFactory)]
pub struct DocumentFactoryWASM(DocumentFactory<ExternalStateRepositoryLikeWrapper>);

#[wasm_bindgen(js_class=DocumentFactory)]
impl DocumentFactoryWASM {
    #[wasm_bindgen(constructor)]
    pub fn new(
        protocol_version: u32,
        document_validator: DocumentValidatorWasm,
        state_repository: ExternalStateRepositoryLike,
    ) -> DocumentFactoryWASM {
        console_error_panic_hook::set_once();
        let factory = DocumentFactory::new(
            protocol_version,
            document_validator.into(),
            DataContractFetcherAndValidator::new(Arc::new(
                ExternalStateRepositoryLikeWrapper::new(state_repository),
            )),
        );

        DocumentFactoryWASM(factory)
    }

    #[wasm_bindgen(js_name=create)]
    pub fn create(
        &self,
        data_contract: DataContractWasm,
        js_owner_id: &JsValue,
        document_type: &str,
        data: &JsValue,
    ) -> Result<DocumentWasm, JsValue> {
        let owner_id = identifier_from_js_value(js_owner_id)?;
        let dynamic_data = data.with_serde_to_json_value()?;
        let document = self
            .0
            .create(
                data_contract.into(),
                owner_id,
                document_type.to_string(),
                dynamic_data,
            )
            .with_js_error()?;

        Ok(document.into())
    }

    #[wasm_bindgen(js_name=createStateTransition)]
    pub fn create_state_transition(
        &self,
        documents_container: DocumentsContainer,
    ) -> Result<DocumentsBatchTransitionWASM, JsValue> {
        let mut documents_container = documents_container;

        let batch_transition = self
            .0
            .create_state_transition([
                (Action::Create, documents_container.take_documents_create()),
                (
                    Action::Replace,
                    documents_container.take_documents_replace(),
                ),
                (Action::Delete, documents_container.take_documents_delete()),
            ])
            .with_js_error()?;

        Ok(batch_transition.into())
    }

    #[wasm_bindgen(js_name=createFromObject)]
    pub async fn create_from_object(
        &self,
        raw_document_js: &JsValue,
        options: &JsValue,
    ) -> Result<DocumentWasm, JsValue> {
        let raw_document = raw_document_js.with_serde_to_json_value()?;
        let options: FactoryOptions = if options.is_object() {
            let raw_options = options.with_serde_to_json_value()?;
            serde_json::from_value(raw_options).with_js_error()?
        } else {
            Default::default()
        };

        let document = self
            .0
            .create_from_object(raw_document, options)
            .await
            .with_js_error()?;

        Ok(document.into())
    }

    #[wasm_bindgen(js_name=createFromBuffer)]
    pub async fn create_from_buffer(
        &self,
        buffer: Vec<u8>,
        options: &JsValue,
    ) -> Result<DocumentWasm, JsValue> {
        let options: FactoryOptions = if options.is_object() {
            let raw_options = options.with_serde_to_json_value()?;
            serde_json::from_value(raw_options).with_js_error()?
        } else {
            Default::default()
        };

        let document = self
            .0
            .create_from_buffer(buffer, options)
            .await
            .with_js_error()?;

        Ok(document.into())
    }
}

impl DocumentFactoryWASM {
    pub(crate) fn into_inner(self) -> DocumentFactory<ExternalStateRepositoryLikeWrapper> {
        self.0
    }
}
