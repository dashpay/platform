#![allow(clippy::from_over_into)]

use std::collections::BTreeMap;
use std::convert::{TryFrom, TryInto};

pub use serde::{Deserialize, Serialize};
use serde_json::Value;
use wasm_bindgen::prelude::*;

use dpp::data_contract::DataContract;
use dpp::util::string_encoding::Encoding;

use crate::errors::{from_dpp_err, RustConversionError};
use crate::identifier::IdentifierWrapper;
use crate::metadata::MetadataWasm;
use crate::{bail_js, with_js_error};

#[wasm_bindgen(js_name=DataContract)]
#[derive(Debug, Clone)]
pub struct DataContractWasm(DataContract);

impl std::convert::From<DataContract> for DataContractWasm {
    fn from(v: DataContract) -> Self {
        DataContractWasm(v)
    }
}

impl std::convert::Into<DataContract> for DataContractWasm {
    fn into(self) -> DataContract {
        self.0
    }
}

#[wasm_bindgen(js_class=DataContract)]
impl DataContractWasm {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        DataContract::default().into()
    }

    #[wasm_bindgen(js_name=getProtocolVersion)]
    pub fn get_protocol_version(&self) -> u32 {
        self.0.protocol_version
    }

    #[wasm_bindgen(js_name=getId)]
    pub fn get_id(&self) -> IdentifierWrapper {
        self.0.id.clone().into()
    }

    #[wasm_bindgen(js_name=getOwnerId)]
    pub fn get_owner_id(&self) -> IdentifierWrapper {
        self.0.owner_id.clone().into()
    }

    #[wasm_bindgen(js_name=getVersion)]
    pub fn get_version(&self) -> u32 {
        self.0.version
    }

    #[wasm_bindgen(js_name=setVersion)]
    pub fn set_version(&mut self, v: u32) {
        self.0.version = v;
    }

    #[wasm_bindgen(js_name=incrementVersion)]
    pub fn increment_version(&mut self) {
        self.0.increment_version()
    }

    #[wasm_bindgen(js_name=getJsonSchemaId)]
    pub fn get_json_schema_id(&self) -> String {
        self.0.id.to_string(Encoding::Base58)
    }

    #[wasm_bindgen(js_name=setJsonMetaSchema)]
    pub fn set_json_meta_schema(&mut self, schema: String) {
        self.0.schema = schema;
    }

    #[wasm_bindgen(js_name=getJsonMetaSchema)]
    pub fn get_json_meta_schema(&self) -> String {
        self.0.schema.clone()
    }
    #[wasm_bindgen(js_name=setDocuments)]
    pub fn set_documents(&self, documents: JsValue) -> Result<(), JsValue> {
        let json_value: Value = with_js_error!(JsValue::into_serde(&documents))?;

        let mut docs: BTreeMap<String, Value> = BTreeMap::new();
        if let Value::Object(o) = json_value {
            for (k, v) in o.into_iter() {
                if !v.is_object() {
                    bail_js!("is not an object")
                }
                docs.insert(k, v);
            }
        } else {
            bail_js!("the parameter 'documents' is not an JS object")
        }
        Ok(())
    }

    #[wasm_bindgen(js_name=getDocuments)]
    pub fn get_documents(&self) -> Result<JsValue, JsValue> {
        with_js_error!(JsValue::from_serde(&self.0.documents))
    }

    #[wasm_bindgen(js_name=isDocumentDefined)]
    pub fn is_document_defined(&self, doc_type: String) -> bool {
        self.0.is_document_defined(&doc_type)
    }

    #[wasm_bindgen(js_name=setDocumentSchema)]
    pub fn set_document_schema(
        &mut self,
        doc_type: String,
        schema: JsValue,
    ) -> Result<(), JsValue> {
        let json_schema: Value = with_js_error!(JsValue::into_serde(&schema))?;
        self.0.documents.insert(doc_type, json_schema);
        Ok(())
    }

    #[wasm_bindgen(js_name=getDocumentSchema)]
    pub fn get_document_schema(&mut self, doc_type: &str) -> Result<JsValue, JsValue> {
        let doc_schema = self.0.get_document_schema(doc_type).map_err(from_dpp_err)?;
        with_js_error!(JsValue::from_serde(doc_schema))
    }

    #[wasm_bindgen(js_name=getDocumentSchemaRef)]
    pub fn get_document_schema_ref(&self, doc_type: &str) -> Result<JsValue, JsValue> {
        with_js_error!(JsValue::from_serde(
            &self
                .0
                .get_document_schema_ref(doc_type)
                .map_err(from_dpp_err)?
        ))
    }

    #[wasm_bindgen(js_name=setDefinitions)]
    pub fn set_definitions(&self, definitions: JsValue) -> Result<(), JsValue> {
        let json_value: Value = with_js_error!(JsValue::into_serde(&definitions))?;
        let mut docs: BTreeMap<String, Value> = BTreeMap::new();
        if let Value::Object(o) = json_value {
            for (k, v) in o.into_iter() {
                // v must be a Object
                if !v.is_object() {
                    bail_js!("{:?} is not an object", v);
                }
                docs.insert(k, v);
            }
        } else {
            bail_js!("the parameter 'definitions' is not an JS object");
        }
        Ok(())
    }

    #[wasm_bindgen(js_name=getDefinitions)]
    pub fn get_definitions(&self) -> Result<JsValue, JsValue> {
        with_js_error!(JsValue::from_serde(&self.0.defs))
    }

    #[wasm_bindgen(js_name=setEntropy)]
    pub fn set_entropy(&mut self, e: Vec<u8>) -> Result<(), JsValue> {
        let entropy: [u8; 32] = e.try_into().map_err(|_| {
            RustConversionError::Error(String::from(
                "unable to turn the data into 32 bytes array of bytes",
            ))
            .to_js_value()
        })?;
        self.0.entropy = entropy;
        Ok(())
    }

    #[wasm_bindgen(js_name=getEntropy)]
    pub fn get_entropy(&mut self) -> Vec<u8> {
        self.0.entropy.to_vec()
    }

    #[wasm_bindgen(js_name=getBinaryProperties)]
    pub fn get_binary_properties(&self, doc_type: &str) -> Result<JsValue, JsValue> {
        with_js_error!(JsValue::from_serde(
            self.0
                .get_binary_properties(doc_type)
                .map_err(from_dpp_err)?
        ))
    }

    #[wasm_bindgen(js_name=getMetadata)]
    pub fn get_metadata(&self) -> Option<MetadataWasm> {
        self.0.metadata.clone().map(Into::into)
    }

    #[wasm_bindgen(js_name=setMetadata)]
    pub fn set_metadata(mut self, metadata: MetadataWasm) -> Self {
        self.0.metadata = Some(metadata.into());
        self
    }

    #[wasm_bindgen(js_name=toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsValue> {
        with_js_error!(JsValue::from_serde(&self.0))
    }

    #[wasm_bindgen(js_name=toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        self.to_object()
    }

    #[wasm_bindgen(js_name=toString)]
    pub fn to_string(&self) -> Result<String, JsValue> {
        return with_js_error!(serde_json::to_string(&self.0));
    }

    #[wasm_bindgen(js_name=toBuffer)]
    pub fn to_buffer(&self) -> Result<Vec<u8>, JsValue> {
        self.0.to_buffer().map_err(from_dpp_err)
    }

    #[wasm_bindgen(js_name=hash)]
    pub fn hash(&self) -> Result<Vec<u8>, JsValue> {
        self.0.hash().map_err(from_dpp_err)
    }

    #[wasm_bindgen(js_name=from)]
    pub fn from(v: JsValue) -> Result<DataContractWasm, JsValue> {
        let json_contract: Value = with_js_error!(v.into_serde())?;
        Ok(DataContract::try_from(json_contract)
            .map_err(from_dpp_err)?
            .into())
    }

    #[wasm_bindgen(js_name=from_buffer)]
    pub fn from_buffer(b: Vec<u8>) -> Result<DataContractWasm, JsValue> {
        Ok(DataContract::from_buffer(b).map_err(from_dpp_err)?.into())
    }

    #[wasm_bindgen(js_name=from_string)]
    pub fn from_string(v: &str) -> Result<DataContractWasm, JsValue> {
        Ok(DataContract::try_from(v).map_err(from_dpp_err)?.into())
    }
}
