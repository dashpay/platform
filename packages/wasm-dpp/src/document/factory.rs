use std::{collections::HashMap, convert::TryFrom, sync::Arc};

use anyhow::anyhow;
use dpp::{
    document::{
        self,
        document_factory::{DocumentFactory, FactoryOptions},
        document_transition::Action,
        fetch_and_validate_data_contract::DataContractFetcherAndValidator,
    },
    prelude::Document,
    state_repository,
    util::json_value::{JsonValueExt, ReplaceWith},
};
use wasm_bindgen::prelude::*;

use crate::{
    console_log,
    document::document_data_to_bytes,
    identifier::identifier_from_js_value,
    state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper},
    utils::{IntoWasm, ToSerdeJSONExt, WithJsError},
    DataContractWasm, DocumentWasm, DocumentsBatchTransitionWASM,
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

impl DocumentFactoryWASM {
    pub(crate) fn new_with_state_repository_wrapper(
        protocol_version: u32,
        document_validator: DocumentValidatorWasm,
        state_repository: ExternalStateRepositoryLikeWrapper,
    ) -> Self {
        let factory = DocumentFactory::new(
            protocol_version,
            document_validator.into(),
            DataContractFetcherAndValidator::new(Arc::new(state_repository)),
        );

        DocumentFactoryWASM(factory)
    }
}

#[wasm_bindgen(js_class=DocumentFactory)]
impl DocumentFactoryWASM {
    #[wasm_bindgen(constructor)]
    pub fn new(
        protocol_version: u32,
        document_validator: DocumentValidatorWasm,
        state_repository: ExternalStateRepositoryLike,
    ) -> DocumentFactoryWASM {
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
    pub fn create_state_transition(
        &self,
        documents: &JsValue,
    ) -> Result<DocumentsBatchTransitionWASM, JsValue> {
        let documents_by_action = extract_documents_by_action(documents)?;
        let batch_transition = self
            .0
            .create_state_transition(documents_by_action)
            .with_js_error()?;

        Ok(batch_transition.into())
    }

    #[wasm_bindgen(js_name=createFromObject)]
    pub async fn create_from_object(
        &self,
        raw_document_js: JsValue,
        options: JsValue,
    ) -> Result<DocumentWasm, JsValue> {
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
            .replace_identifier_paths(document::IDENTIFIER_FIELDS, ReplaceWith::Bytes)
            .with_js_error();

        let mut document = self
            .0
            .create_from_object(raw_document, options)
            .await
            .with_js_error()?;

        document_data_to_bytes(&mut document)?;
        Ok(document.into())
    }

    #[wasm_bindgen(js_name=createFromBuffer)]
    pub async fn create_from_buffer(
        &self,
        buffer: Vec<u8>,
        options: &JsValue,
    ) -> Result<DocumentWasm, JsValue> {
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

fn extract_documents_by_action(
    documents: &JsValue,
) -> Result<HashMap<Action, Vec<Document>>, JsValue> {
    let mut documents_by_action: HashMap<Action, Vec<Document>> = Default::default();

    let documents_create = extract_documents_of_action(documents, "create").with_js_error()?;
    let documents_replace = extract_documents_of_action(documents, "replace").with_js_error()?;
    let documents_delete = extract_documents_of_action(documents, "delete").with_js_error()?;

    documents_by_action.insert(Action::Create, documents_create);
    documents_by_action.insert(Action::Replace, documents_replace);
    documents_by_action.insert(Action::Delete, documents_delete);

    Ok(documents_by_action)
}

fn extract_documents_of_action(
    documents: &JsValue,
    action: &str,
) -> Result<Vec<Document>, anyhow::Error> {
    let mut extracted_documents: Vec<Document> = vec![];
    let documents_with_action =
        js_sys::Reflect::get(documents, &action.to_string().into()).unwrap_or(JsValue::NULL);
    if documents_with_action.is_null() || documents_with_action.is_undefined() {
        return Ok(extracted_documents);
    }
    let documents_array = js_sys::Array::try_from(documents_with_action)
        .map_err(|e| anyhow!("property '{}' isn't an array: {}", action, e))?;

    for js_document in documents_array.iter() {
        let document: Document = js_document
            .to_wasm::<DocumentWasm>("Document")
            .map_err(|e| {
                anyhow!(
                    "Element in '{}' isn't a Document instance: {:#?}",
                    action,
                    e
                )
            })?
            .clone()
            .into();
        extracted_documents.push(document)
    }

    Ok(extracted_documents)
}
