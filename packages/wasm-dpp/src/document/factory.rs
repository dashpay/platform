use std::{
    collections::{hash_map::Entry, HashMap},
    convert::TryFrom,
};

use dpp::{
    dashcore::anyhow::Context,
    document::{self, document_factory::DocumentFactory, document_transition::Action},
    prelude::{DataContract, Document, Identifier},
};
use itertools::Itertools;
use js_sys::Array;
use wasm_bindgen::{
    convert::{FromWasmAbi, IntoWasmAbi, RefFromWasmAbi, ReturnWasmAbi},
    prelude::*,
};
use web_sys::console::{log_1, log_2};

use crate::{
    bail_js, console_log,
    errors::RustConversionError,
    identifier::IdentifierWrapper,
    utils::{ToSerdeJSONExt, WithJsError},
    DataContractWasm, DocumentWasm, DocumentsBatchTransitionWASM,
};

use super::validator::DocumentValidatorWasm;

#[wasm_bindgen(js_name = DocumentFactory)]
pub struct DocumentFactoryWASM(DocumentFactory);

#[wasm_bindgen(js_class=DocumentFactory)]
impl DocumentFactoryWASM {
    #[wasm_bindgen(constructor)]
    pub fn new(
        protocol_version: u32,
        document_validator: DocumentValidatorWasm,
        fetch_and_validate_data_contract: JsValue, // TODO
    ) -> DocumentFactoryWASM {
        console_error_panic_hook::set_once();
        let factory = DocumentFactory::new(
            protocol_version,
            document_validator.into(),
            // TODO
            dpp::mocks::FetchAndValidateDataContract {},
        );

        DocumentFactoryWASM(factory)
    }

    #[wasm_bindgen(js_name=create)]
    pub fn create(
        &self,
        data_contract: DataContractWasm,
        owner_id: IdentifierWrapper,
        document_type: &str,
        data: &JsValue,
    ) -> Result<DocumentWasm, JsValue> {
        let dynamic_data = data.with_serde_to_json_value()?;
        let document = self
            .0
            .create(
                data_contract.into(),
                owner_id.inner(),
                document_type.to_string(),
                dynamic_data,
            )
            .with_js_error()?;

        Ok(document.into())
    }

    #[wasm_bindgen(js_name=createStateTransition)]
    pub fn create_state_transition(
        &self,
        js_documents: JsValue,
        data_contract: &DataContractWasm,
    ) -> Result<DocumentsBatchTransitionWASM, JsValue> {
        let mut documents_iter: HashMap<Action, Vec<Document>> = Default::default();

        let js_documents_object = js_sys::Object::try_from(&js_documents)
            .context("expected object")
            .with_js_error()?;

        for entry in js_sys::Object::entries(js_documents_object).iter() {
            let e = Array::from(&entry);
            let mut entry_iter = e.iter();

            let action = entry_iter
                .next()
                .context("key isn't present")
                .with_js_error()?
                .as_string()
                .context("the key must be a string")
                .with_js_error()?;

            let js_documents_object = entry_iter
                .next()
                .context("value isn't present")
                .with_js_error()?;

            let js_documents: Vec<JsValue> = js_sys::try_iter(&js_documents_object)?
                .context("documents cannot be  none")
                .with_js_error()?
                .try_collect()?;

            let action = Action::try_from(action).with_js_error()?;
            let documents: Vec<DocumentWasm> = js_documents
                .into_iter()
                .map(|v| DocumentWasm::new(v, data_contract))
                .try_collect()?;

            match documents_iter.entry(action) {
                Entry::Occupied(ref mut o) => {
                    o.get_mut().extend(documents.into_iter().map(|v| v.0))
                }
                Entry::Vacant(v) => {
                    v.insert(documents.into_iter().map(|v| v.0).collect_vec());
                }
            };
        }

        let batch_transition = self
            .0
            .create_state_transition(documents_iter)
            .with_js_error()?;

        Ok(batch_transition.into())
    }
}

impl DocumentFactoryWASM {
    pub fn inner(self) -> DocumentFactory {
        self.0
    }
}

#[wasm_bindgen(js_name=FactoryInput)]
pub struct CreateTransitionInput {
    create: Vec<DocumentWasm>,
    replace: Vec<DocumentWasm>,
    delete: Vec<DocumentWasm>,
}
