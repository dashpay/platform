use std::sync::Arc;

use wasm_bindgen::{prelude::*, JsValue};

use crate::{
    fetch_and_validate_data_contract::DataContractFetcherAndValidatorWasm,
    utils::{get_class_name, IntoWasm},
    validation::ValidationResultWasm,
    DataContractWasm, DocumentFactoryWASM, DocumentValidatorWasm, DocumentWasm,
    DocumentsBatchTransitionWasm,
};

#[derive(Clone)]
#[wasm_bindgen(js_name=DocumentFacade)]
pub struct DocumentFacadeWasm {
    validator: Arc<DocumentValidatorWasm>,
    factory: Arc<DocumentFactoryWASM>,
    data_contract_fetcher_and_validator: Arc<DataContractFetcherAndValidatorWasm>,
}

impl DocumentFacadeWasm {
    pub fn new_with_arc(
        document_validator: Arc<DocumentValidatorWasm>,
        document_factory: Arc<DocumentFactoryWASM>,
        data_contract_fetcher_and_validator: Arc<DataContractFetcherAndValidatorWasm>,
    ) -> Self {
        Self {
            validator: document_validator,
            factory: document_factory,
            data_contract_fetcher_and_validator,
        }
    }
}

#[wasm_bindgen(js_class=DocumentFacade)]
impl DocumentFacadeWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(
        document_validator: DocumentValidatorWasm,
        document_factory: DocumentFactoryWASM,
        data_contract_fetcher_and_validator: DataContractFetcherAndValidatorWasm,
    ) -> Self {
        Self {
            validator: Arc::new(document_validator),
            factory: Arc::new(document_factory),
            data_contract_fetcher_and_validator: Arc::new(data_contract_fetcher_and_validator),
        }
    }

    #[wasm_bindgen(js_name=create)]
    pub fn create(
        &self,
        data_contract: &DataContractWasm,
        js_owner_id: &JsValue,
        document_type: &str,
        data: &JsValue,
    ) -> Result<DocumentWasm, JsValue> {
        self.factory
            .create(data_contract, js_owner_id, document_type, data)
    }

    /// Creates Document from object
    #[wasm_bindgen(js_name=createFromObject)]
    pub async fn create_from_object(
        &self,
        raw_document: JsValue,
        options: JsValue,
    ) -> Result<DocumentWasm, JsValue> {
        self.factory.create_from_object(raw_document, options).await
    }

    /// Creates Document form bytes
    #[wasm_bindgen(js_name=createFromBuffer)]
    pub async fn create_from_buffer(
        &self,
        bytes: Vec<u8>,
        options: JsValue,
    ) -> Result<DocumentWasm, JsValue> {
        self.factory.create_from_buffer(bytes, &options).await
    }

    /// Creates Documents State Transition
    #[wasm_bindgen(js_name=createStateTransition)]
    pub fn create_state_transition(
        &self,
        documents: &JsValue, // documents_container: DocumentsContainer,
    ) -> Result<DocumentsBatchTransitionWasm, JsValue> {
        self.factory.create_state_transition(documents)
    }

    /// Creates Documents State Transition
    #[wasm_bindgen(js_name=validate)]
    pub async fn validate_document(
        &self,
        document: &JsValue,
    ) -> Result<ValidationResultWasm, JsValue> {
        let raw_document = if get_class_name(document) == "Document" {
            let document = document.to_wasm::<DocumentWasm>("Document")?;
            document.to_object(&JsValue::NULL)?
        } else {
            document.to_owned()
        };

        self.validate_raw_document(raw_document).await
    }

    /// Creates Documents State Transition
    pub async fn validate_raw_document(
        &self,
        js_raw_document: JsValue,
    ) -> Result<ValidationResultWasm, JsValue> {
        let result = self
            .data_contract_fetcher_and_validator
            .validate(&js_raw_document)
            .await?;
        if !result.is_valid() {
            return Ok(result);
        }
        let data_contract = result
            .get_data()
            .to_wasm::<DataContractWasm>("DataContract")?;

        self.validator.validate(&js_raw_document, &data_contract)
    }
}
