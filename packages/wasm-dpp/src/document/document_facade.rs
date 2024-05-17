use std::rc::Rc;
use wasm_bindgen::{prelude::*, JsValue};

use crate::document::factory::DocumentFactoryWASM;
use crate::{DataContractWasm, ExtendedDocumentWasm};

use crate::document::state_transition::document_batch_transition::DocumentsBatchTransitionWasm;

#[derive(Clone)]
#[wasm_bindgen(js_name=DocumentFacade)]
pub struct DocumentFacadeWasm {
    // validator: Arc<DocumentValidatorWasm>,
    factory: Rc<DocumentFactoryWASM>,
    // data_contract_fetcher_and_validator: Arc<DataContractFetcherAndValidatorWasm>,
}

impl DocumentFacadeWasm {
    pub fn new_with_arc(
        // document_validator: Arc<DocumentValidatorWasm>,
        document_factory: Rc<DocumentFactoryWASM>,
        // data_contract_fetcher_and_validator: Arc<DataContractFetcherAndValidatorWasm>,
    ) -> Self {
        Self {
            factory: document_factory,
            // data_contract_fetcher_and_validator,
        }
    }
}

#[wasm_bindgen(js_class=DocumentFacade)]
impl DocumentFacadeWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(document_factory: DocumentFactoryWASM) -> Self {
        Self {
            factory: Rc::new(document_factory),
        }
    }

    #[wasm_bindgen(js_name=create)]
    pub fn create(
        &self,
        data_contract: &DataContractWasm,
        js_owner_id: &JsValue,
        document_type: &str,
        data: &JsValue,
    ) -> Result<ExtendedDocumentWasm, JsValue> {
        self.factory
            .create(data_contract, js_owner_id, document_type, data)
    }
    //
    // /// Creates Document from object
    // #[wasm_bindgen(js_name=createFromObject)]
    // pub async fn create_from_object(
    //     &self,
    //     raw_document: JsValue,
    //     options: Option<js_sys::Object>,
    // ) -> Result<ExtendedDocumentWasm, JsValue> {
    //     self.factory
    //         .create_from_object(
    //             raw_document,
    //             options.map(Into::into).unwrap_or(JsValue::undefined()),
    //         )
    //         .await
    // }
    //
    // /// Creates Document form bytes
    // #[wasm_bindgen(js_name=createFromBuffer)]
    // pub async fn create_from_buffer(
    //     &self,
    //     bytes: Vec<u8>,
    //     options: Option<js_sys::Object>,
    // ) -> Result<ExtendedDocumentWasm, JsValue> {
    //     self.factory
    //         .create_from_buffer(
    //             bytes,
    //             &options.map(Into::into).unwrap_or(JsValue::undefined()),
    //         )
    //         .await
    // }

    // TODO(rs-drive-abci): add tests
    #[wasm_bindgen(js_name=createExtendedDocumentFromDocumentBuffer)]
    pub fn create_extended_from_document_buffer(
        &self,
        buffer: Vec<u8>,
        document_type: &str,
        data_contract: &DataContractWasm,
    ) -> Result<ExtendedDocumentWasm, JsValue> {
        self.factory
            .create_extended_from_document_buffer(buffer, document_type, data_contract)
    }

    /// Creates Documents State Transition
    #[wasm_bindgen(js_name=createStateTransition)]
    pub fn create_state_transition(
        &self,
        documents: &JsValue,
        nonce_counter_value: &js_sys::Object, //IdentityID/ContractID -> nonce
    ) -> Result<DocumentsBatchTransitionWasm, JsValue> {
        self.factory
            .create_state_transition(documents, nonce_counter_value)
    }

    // /// Creates Documents State Transition
    // #[wasm_bindgen(js_name=validate)]
    // pub async fn validate_document(
    //     &self,
    //     document: &JsValue,
    // ) -> Result<ValidationResultWasm, JsValue> {
    //     let raw_document = if get_class_name(document) == "ExtendedDocument" {
    //         let document = document.to_wasm::<ExtendedDocumentWasm>("ExtendedDocument")?;
    //         document.to_object(&JsValue::NULL)?
    //     } else {
    //         document.to_owned()
    //     };
    //
    //     self.validate_raw_document(raw_document).await
    // }
    //
    // /// Creates Documents State Transition
    // pub async fn validate_raw_document(
    //     &self,
    //     js_raw_document: JsValue,
    // ) -> Result<ValidationResultWasm, JsValue> {
    //     let result = self
    //         .data_contract_fetcher_and_validator
    //         .validate(&js_raw_document)
    //         .await?;
    //     if !result.is_valid() {
    //         return Ok(result);
    //     }
    //     let data_contract = result
    //         .get_data()
    //         .to_wasm::<DataContractWasm>("DataContract")?;
    //
    //     self.validator.validate(&js_raw_document, &data_contract)
    // }
}
