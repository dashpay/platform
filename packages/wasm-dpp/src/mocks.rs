use dpp::prelude::Identifier;
use dpp::util::json_schema::JsonSchemaExt;
use serde::Serialize;
use wasm_bindgen::prelude::*;

use dpp::document::document_transition::{
    DocumentTransition, DocumentTransitionExt, DocumentTransitionObjectLike,
};
use dpp::errors::consensus::ConsensusError as DPPConsensusError;

use crate::buffer::Buffer;
use crate::document_batch_transition::document_transition::{
    DocumentCreateTransitionWasm, DocumentDeleteTransitionWasm, DocumentReplaceTransitionWasm,
};
use crate::identifier::IdentifierWrapper;
use crate::utils::WithJsError;
use crate::{with_js_error, BinaryType, DataContractWasm};

#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct DocumentTransitionWasm(DocumentTransition);

#[wasm_bindgen(js_class=DocumentTransitionWasm)]
impl DocumentTransitionWasm {
    #[wasm_bindgen(js_name=getId)]
    pub fn get_id(&self) -> IdentifierWrapper {
        self.0.get_id().to_owned().into()
    }

    #[wasm_bindgen(js_name=getType)]
    pub fn get_type(&self) -> String {
        self.0.get_document_type().to_owned()
    }

    #[wasm_bindgen(js_name=getAction)]
    pub fn get_action(&self) -> u8 {
        self.0.get_action().into()
    }

    #[wasm_bindgen(js_name=getDataContract)]
    pub fn get_data_contract(&self) -> DataContractWasm {
        self.0.get_data_contract().to_owned().into()
    }

    #[wasm_bindgen(js_name=getDataContractId)]
    pub fn get_data_contract_id(&self) -> IdentifierWrapper {
        self.0.get_data_contract_id().to_owned().into()
    }

    #[wasm_bindgen(js_name=get)]
    pub fn get(&self, path: &str) -> JsValue {
        let binary_type = self.get_binary_type_of_path(path);

        if let Some(value) = self.0.get_dynamic_property(path) {
            match binary_type {
                BinaryType::Identifier => {
                    if let Ok(bytes) = serde_json::from_value::<Vec<u8>>(value.to_owned()) {
                        let id: IdentifierWrapper = Identifier::from_bytes(&bytes).unwrap().into();

                        return id.into();
                    }
                }
                BinaryType::Buffer => {
                    if let Ok(bytes) = serde_json::from_value::<Vec<u8>>(value.to_owned()) {
                        return Buffer::from_bytes(&bytes).into();
                    }
                }
                BinaryType::None => {
                    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
                    if let Ok(js_value) = value.serialize(&serializer) {
                        return js_value;
                    }
                }
            }
        }

        JsValue::undefined()
    }

    #[wasm_bindgen(js_name=getObject)]
    pub fn to_object(&self, options: &JsValue) -> Result<JsValue, JsValue> {
        // TODO options??

        match self.0 {
            DocumentTransition::Create(ref t) => {
                DocumentCreateTransitionWasm::from(t.to_owned()).to_object()
            }
            DocumentTransition::Replace(ref t) => {
                DocumentReplaceTransitionWasm::from(t.to_owned()).to_object()
            }
            DocumentTransition::Delete(ref t) => {
                DocumentDeleteTransitionWasm::from(t.to_owned()).to_object()
            }
        }
    }

    #[wasm_bindgen(js_name=toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        let json_value = self.0.to_json().with_js_error()?;
        with_js_error!(json_value.serialize(&serde_wasm_bindgen::Serializer::json_compatible()))
    }
}

impl DocumentTransitionWasm {
    fn get_binary_type_of_path(&self, path: impl AsRef<str>) -> BinaryType {
        let maybe_binary_properties = self
            .0
            .get_data_contract()
            .get_binary_properties(self.0.get_document_type());

        if let Ok(binary_properties) = maybe_binary_properties {
            if let Some(data) = binary_properties.get(path.as_ref()) {
                if data.is_type_of_identifier() {
                    return BinaryType::Identifier;
                }
                return BinaryType::Buffer;
            }
        }
        BinaryType::None
    }
}

impl From<DocumentTransition> for DocumentTransitionWasm {
    fn from(v: DocumentTransition) -> Self {
        DocumentTransitionWasm(v)
    }
}

#[derive(Debug)]
pub struct ConsensusError {}

pub fn from_consensus_to_js_error(_: DPPConsensusError) -> JsValue {
    unimplemented!()
}
