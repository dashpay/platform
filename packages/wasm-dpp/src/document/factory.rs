use std::collections::HashSet;
use std::sync::Arc;

use dpp::platform_value::btreemap_field_replacement::BTreeValueMapInsertionPathHelper;
use dpp::platform_value::ReplacementType;
use dpp::{
    document::{
        document_factory::{DocumentFactory, FactoryOptions},
        document_transition::Action,
        extended_document,
        fetch_and_validate_data_contract::DataContractFetcherAndValidator,
    },
    util::json_value::{JsonValueExt, ReplaceWith},
    ProtocolError,
};
use wasm_bindgen::prelude::*;

use crate::{
    identifier::identifier_from_js_value,
    state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper},
    utils::{ToSerdeJSONExt, WithJsError},
    DataContractWasm, DocumentsBatchTransitionWASM, DocumentsContainer, ExtendedDocumentWasm,
};

use super::validator::DocumentValidatorWasm;

#[wasm_bindgen(js_name=DocumentTransitions)]
#[derive(Debug, Default)]
pub struct DocumentTransitions {
    create: Vec<ExtendedDocumentWasm>,
    replace: Vec<ExtendedDocumentWasm>,
    delete: Vec<ExtendedDocumentWasm>,
}

#[wasm_bindgen(js_class=DocumentTransitions)]
impl DocumentTransitions {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Default::default()
    }

    #[wasm_bindgen(js_name = "addTransitionCreate")]
    pub fn add_transition_create(&mut self, transition: ExtendedDocumentWasm) {
        self.create.push(transition)
    }

    #[wasm_bindgen(js_name = "addTransitionReplace")]
    pub fn add_transition_replace(&mut self, transition: ExtendedDocumentWasm) {
        self.replace.push(transition)
    }

    #[wasm_bindgen(js_name = "addTransitionDelete")]
    pub fn add_transition_delete(&mut self, transition: ExtendedDocumentWasm) {
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
            None,
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
    ) -> Result<ExtendedDocumentWasm, JsValue> {
        let owner_id = identifier_from_js_value(js_owner_id)?;
        let dynamic_data = data.with_serde_to_json_value()?;
        let document = self
            .0
            .create_document_for_state_transition(
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
        raw_document_js: JsValue,
        options: JsValue,
    ) -> Result<ExtendedDocumentWasm, JsValue> {
        let mut raw_document = raw_document_js.with_serde_to_json_value()?;
        let options: FactoryOptions = if !options.is_undefined() && options.is_object() {
            let raw_options = options.with_serde_to_json_value()?;
            serde_json::from_value(raw_options).with_js_error()?
        } else {
            Default::default()
        };

        // Errors are ignored. When `Buffer` crosses the WASM boundary it becomes an Array.
        // When `Identifier` crosses the WASM boundary, it becomes a String. From perspective of JS
        // `Identifier` and `Buffer` are used interchangeably, so we we can expect the replacing may fail when `Buffer` is provided
        let _ = raw_document
            .replace_identifier_paths(extended_document::IDENTIFIER_FIELDS, ReplaceWith::Bytes)
            .with_js_error();

        let mut document = self
            .0
            .create_from_object(raw_document, options)
            .await
            .with_js_error()?;
        let (identifier_paths, binary_paths) = document
            .get_identifiers_and_binary_paths_owned()
            .with_js_error()?;
        // When data contract is available, replace remaining dynamic paths
        let mut document_data = document.properties_as_mut();
        document_data
            .replace_at_paths(identifier_paths, ReplacementType::Bytes)
            .map_err(ProtocolError::ValueError)
            .with_js_error()?;
        document_data
            .replace_at_paths(binary_paths, ReplacementType::Bytes)
            .map_err(ProtocolError::ValueError)
            .with_js_error()?;

        Ok(document.into())
    }

    #[wasm_bindgen(js_name=createFromBuffer)]
    pub async fn create_from_buffer(
        &self,
        buffer: Vec<u8>,
        options: &JsValue,
    ) -> Result<ExtendedDocumentWasm, JsValue> {
        let options: FactoryOptions = if !options.is_undefined() && options.is_object() {
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
