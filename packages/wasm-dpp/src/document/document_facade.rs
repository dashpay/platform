use wasm_bindgen::{prelude::*, JsValue};

use crate::{
    fetch_and_validate_data_contract::DataContractFetcherAndValidatorWasm, utils::IntoWasm,
    validation::ValidationResultWasm, DataContractWasm, DocumentFactoryWASM, DocumentValidatorWasm,
    DocumentWasm, DocumentsBatchTransitionWASM, DocumentsContainer,
};

impl DocumentFacadeWasm {
    pub fn new(
        document_validator: DocumentValidatorWasm,
        document_factory: DocumentFactoryWASM,
        data_contract_fetcher_and_validator: DataContractFetcherAndValidatorWasm,
    ) -> Self {
        Self {
            validator: document_validator,
            factory: document_factory,
            data_contract_fetcher_and_validator,
        }
    }
}

#[wasm_bindgen(js_name=DocumentFacade)]
pub struct DocumentFacadeWasm {
    validator: DocumentValidatorWasm,
    factory: DocumentFactoryWASM,
    data_contract_fetcher_and_validator: DataContractFetcherAndValidatorWasm,
}

#[wasm_bindgen(js_class=DocumentFacade)]
impl DocumentFacadeWasm {
    #[wasm_bindgen(js_name=create)]
    pub fn create(
        &self,
        data_contract: DataContractWasm,
        js_owner_id: &JsValue,
        document_type: &str,
        data: &JsValue,
    ) -> Result<DocumentWasm, JsValue> {
        self.factory
            .create(data_contract, js_owner_id, document_type, data)
    }

    /// Creates Document from object
    pub async fn create_from_object(
        &self,
        raw_document: JsValue,
        options: JsValue,
    ) -> Result<DocumentWasm, JsValue> {
        self.factory.create_from_object(raw_document, options).await
    }

    /// Creates Document form bytes
    pub async fn create_from_buffer(
        &self,
        bytes: Vec<u8>,
        options: JsValue,
    ) -> Result<DocumentWasm, JsValue> {
        self.factory.create_from_buffer(bytes, &options).await
    }

    /// Creates Documents State Transition
    pub fn create_state_transition(
        &self,
        documents_container: DocumentsContainer,
    ) -> Result<DocumentsBatchTransitionWASM, JsValue> {
        self.factory.create_state_transition(documents_container)
    }

    /// Creates Documents State Transition
    pub async fn validate_document(
        &self,
        document: &DocumentWasm,
    ) -> Result<ValidationResultWasm, JsValue> {
        let raw_document = document.to_object(&JsValue::null())?;
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

        let data_contract = result
            .get_data()
            .to_wasm::<DataContractWasm>("DataContract")?;
        self.validator.validate(&js_raw_document, &data_contract)
    }
}
