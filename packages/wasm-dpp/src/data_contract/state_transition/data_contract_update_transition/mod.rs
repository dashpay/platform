mod apply;
mod validation;

use std::collections::HashMap;

pub use apply::*;
pub use validation::*;

use dpp::{
    data_contract::state_transition::data_contract_update_transition::DataContractUpdateTransition,
    platform_value,
    state_transition::{
        StateTransitionConvert, StateTransitionIdentitySigned, StateTransitionLike,
    },
    ProtocolError,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::utils::WithJsError;
use crate::{
    buffer::Buffer, errors::protocol_error::from_protocol_error, identifier::IdentifierWrapper,
    with_js_error, DataContractParameters, DataContractWasm,
};

#[derive(Clone)]
#[wasm_bindgen(js_name=DataContractUpdateTransition)]
pub struct DataContractUpdateTransitionWasm(DataContractUpdateTransition);

impl From<DataContractUpdateTransition> for DataContractUpdateTransitionWasm {
    fn from(v: DataContractUpdateTransition) -> Self {
        DataContractUpdateTransitionWasm(v)
    }
}

impl From<DataContractUpdateTransitionWasm> for DataContractUpdateTransition {
    fn from(val: DataContractUpdateTransitionWasm) -> Self {
        val.0
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DataContractUpdateTransitionParameters {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    data_contract: Option<DataContractParameters>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    entropy: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    signature: Option<Vec<u8>>,
    #[serde(flatten)]
    extra: HashMap<String, serde_json::Value>,
}

#[wasm_bindgen(js_class=DataContractUpdateTransition)]
impl DataContractUpdateTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(raw_parameters: JsValue) -> Result<DataContractUpdateTransitionWasm, JsValue> {
        let parameters: DataContractUpdateTransitionParameters =
            with_js_error!(serde_wasm_bindgen::from_value(raw_parameters))?;
        let raw_data_contract_update_transition = platform_value::to_value(parameters)
            .map_err(ProtocolError::ValueError)
            .with_js_error()?;
        DataContractUpdateTransition::from_raw_object(raw_data_contract_update_transition)
            .map(Into::into)
            .with_js_error()
    }

    #[wasm_bindgen(js_name=getDataContract)]
    pub fn get_data_contract(&self) -> DataContractWasm {
        self.0.data_contract.clone().into()
    }

    #[wasm_bindgen(js_name=getProtocolVersion)]
    pub fn get_protocol_version(&self) -> u32 {
        self.0.protocol_version
    }

    #[wasm_bindgen(js_name=getEntropy)]
    pub fn get_entropy(&self) -> Buffer {
        Buffer::from_bytes_owned(self.0.data_contract.entropy.to_vec())
    }

    #[wasm_bindgen(js_name=getOwnerId)]
    pub fn get_owner_id(&self) -> IdentifierWrapper {
        (*self.0.get_owner_id()).into()
    }

    #[wasm_bindgen(js_name=getType)]
    pub fn get_type(&self) -> u32 {
        self.0.get_type() as u32
    }

    #[wasm_bindgen(js_name=toJSON)]
    pub fn to_json(&self, skip_signature: Option<bool>) -> Result<JsValue, JsValue> {
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        Ok(self
            .0
            .to_json(skip_signature.unwrap_or(false))
            .with_js_error()?
            .serialize(&serializer)
            .expect("JSON is a valid object"))
    }

    #[wasm_bindgen(js_name=toBuffer)]
    pub fn to_buffer(&self, skip_signature: Option<bool>) -> Result<Buffer, JsValue> {
        let bytes = self
            .0
            .to_cbor_buffer(skip_signature.unwrap_or(false))
            .with_js_error()?;
        Ok(Buffer::from_bytes(&bytes))
    }

    #[wasm_bindgen(js_name=getModifiedDataIds)]
    pub fn get_modified_data_ids(&self) -> Vec<JsValue> {
        self.0
            .get_modified_data_ids()
            .into_iter()
            .map(|identifier| Into::<IdentifierWrapper>::into(identifier).into())
            .collect()
    }

    #[wasm_bindgen(js_name=isDataContractStateTransition)]
    pub fn is_data_contract_state_transition(&self) -> bool {
        self.0.is_data_contract_state_transition()
    }

    #[wasm_bindgen(js_name=isDocumentStateTransition)]
    pub fn is_document_state_transition(&self) -> bool {
        self.0.is_document_state_transition()
    }

    #[wasm_bindgen(js_name=isIdentityStateTransition)]
    pub fn is_identity_state_transition(&self) -> bool {
        self.0.is_identity_state_transition()
    }

    #[wasm_bindgen(js_name=hash)]
    pub fn hash(&self, skip_signature: Option<bool>) -> Result<Buffer, JsValue> {
        let bytes = self
            .0
            .hash(skip_signature.unwrap_or(false))
            .with_js_error()?;
        Ok(Buffer::from_bytes(&bytes))
    }

    #[wasm_bindgen(js_name=toObject)]
    pub fn to_object(&self, skip_signature: Option<bool>) -> Result<JsValue, JsValue> {
        let serde_object = self
            .0
            .to_object(skip_signature.unwrap_or(false))
            .map_err(from_protocol_error)?;
        serde_object
            .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
            .map_err(|e| e.into())
    }
}
