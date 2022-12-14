use dpp::{
    dashcore::anyhow::Context, document::DocumentsBatchTransition, prelude::DataContract,
    state_transition::StateTransitionConvert,
};
use itertools::Itertools;
use js_sys::Array;
use serde::Serialize;
use wasm_bindgen::{
    convert::{FromWasmAbi, IntoWasmAbi, RefFromWasmAbi},
    prelude::*,
};
use web_sys::console::log_1;

use crate::{
    utils::{stringify, ToSerdeJSONExt, WithJsError},
    with_js_error, DataContractWasm,
};
pub mod document_transition;

#[derive(Debug)]
#[wasm_bindgen(js_name = DocumentsBatchTransition)]
pub struct DocumentsBatchTransitionWASM(DocumentsBatchTransition);

#[wasm_bindgen(js_class=DocumentsBatchTransition)]
impl DocumentsBatchTransitionWASM {
    #[wasm_bindgen(constructor)]
    pub fn from_raw_object(
        js_raw_transition: JsValue,
        js_data_contracts: JsValue, // TODO decide if it should be a reference or the whole object
    ) -> Result<DocumentsBatchTransitionWASM, JsValue> {
        let js_data_contracts_array = Array::from(&js_data_contracts);
        let data_contracts = js_data_contracts_array
            .iter()
            .map(|v| unsafe { DataContractWasm::from_abi(v.into_abi()) })
            .collect_vec();

        let json_value = js_raw_transition.with_serde_to_json_value();
        log_1(&format!("this is  the json value: {json_value:#?}").into());

        // let raw_transition_str = stringify(&raw_transition);

        // raw object <- this is the problem

        // if we expect the buffers -> this some values must be converted into the

        // we should rather use from_json_object

        // now we need to create

        // let id = unsafe { IdentifierWrapper::from_abi(js_value_to_set.into_abi()) };
        // now we should convert the JsValue into the WASM

        // convert into the

        // here we should start conversion into the

        todo!()
    }

    #[wasm_bindgen(js_name=toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        let value = self.0.to_json().with_js_error()?;
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();

        with_js_error!(value.serialize(&serializer))
    }
}

impl From<DocumentsBatchTransition> for DocumentsBatchTransitionWASM {
    fn from(t: DocumentsBatchTransition) -> Self {
        DocumentsBatchTransitionWASM(t)
    }
}
