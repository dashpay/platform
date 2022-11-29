#![allow(clippy::from_over_into)]

use std::collections::BTreeMap;
use std::convert::{TryFrom, TryInto};

pub use serde::{Deserialize, Serialize};
use serde_json::Value;
use wasm_bindgen::prelude::*;

use dpp::data_contract::{DataContract, SCHEMA_URI};
use dpp::util::string_encoding::Encoding;

use crate::errors::{from_dpp_err, RustConversionError};
use crate::metadata::MetadataWasm;
use crate::{bail_js, with_js_error};
use crate::{buffer::Buffer, identifier::IdentifierWrapper};

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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DataContractParameters {
    #[serde(rename = "$schema")]
    schema: String,
    #[serde(rename = "$id")]
    id: Vec<u8>,
    owner_id: Vec<u8>,
    documents: serde_json::Value,
    #[serde(rename = "$defs")]
    defs: serde_json::Value,
    protocol_version: u32,
    version: u32,
}

#[wasm_bindgen(js_class=DataContract)]
impl DataContractWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(raw_parameters: JsValue) -> Result<DataContractWasm, JsValue> {
        let parameters: DataContractParameters =
            with_js_error!(serde_wasm_bindgen::from_value(raw_parameters))?;

        DataContract::from_raw_object(
            serde_json::to_value(parameters).expect("Implements Serialize"),
        )
        .map_err(from_dpp_err)
        .map(Into::into)
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
    pub fn set_documents(&mut self, documents: JsValue) -> Result<(), JsValue> {
        let json_value: Value = with_js_error!(serde_wasm_bindgen::from_value(documents))?;

        let mut docs: BTreeMap<String, Value> = BTreeMap::new();
        if let Value::Object(o) = json_value {
            for (k, v) in o.into_iter() {
                if !v.is_object() {
                    bail_js!("is not an object")
                }
                docs.insert(k, v);
            }
            self.0.documents = docs;
        } else {
            bail_js!("the parameter 'documents' is not an JS object")
        }
        Ok(())
    }

    #[wasm_bindgen(js_name=getDocuments)]
    pub fn get_documents(&self) -> Result<JsValue, JsValue> {
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        with_js_error!(self.0.documents.serialize(&serializer))
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
        let json_schema: Value = with_js_error!(serde_wasm_bindgen::from_value(schema))?;
        self.0.set_document_schema(doc_type, json_schema);
        Ok(())
    }

    #[wasm_bindgen(js_name=getDocumentSchema)]
    pub fn get_document_schema(&mut self, doc_type: &str) -> Result<JsValue, JsValue> {
        let doc_schema = self.0.get_document_schema(doc_type).map_err(from_dpp_err)?;
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        with_js_error!(doc_schema.serialize(&serializer))
    }

    #[wasm_bindgen(js_name=getDocumentSchemaRef)]
    pub fn get_document_schema_ref(&self, doc_type: &str) -> Result<JsValue, JsValue> {
        with_js_error!(serde_wasm_bindgen::to_value(
            &self
                .0
                .get_document_schema_ref(doc_type)
                .map_err(from_dpp_err)?
        ))
    }

    #[wasm_bindgen(js_name=setDefinitions)]
    pub fn set_definitions(&mut self, definitions: JsValue) -> Result<(), JsValue> {
        let json_value: Value = with_js_error!(serde_wasm_bindgen::from_value(definitions))?;
        let mut definitions: BTreeMap<String, Value> = BTreeMap::new();
        if let Value::Object(o) = json_value {
            for (k, v) in o.into_iter() {
                // v must be a Object
                if !v.is_object() {
                    bail_js!("{:?} is not an object", v);
                }
                definitions.insert(k, v);
            }
            self.0.defs = definitions;
        } else {
            bail_js!("the parameter 'definitions' is not an JS object");
        }
        Ok(())
    }

    #[wasm_bindgen(js_name=getDefinitions)]
    pub fn get_definitions(&self) -> Result<JsValue, JsValue> {
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        with_js_error!(self.0.defs.serialize(&serializer))
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
    pub fn get_entropy(&mut self) -> Buffer {
        Buffer::from_bytes(&self.0.entropy)
    }

    #[wasm_bindgen(js_name=getBinaryProperties)]
    pub fn get_binary_properties(&self, doc_type: &str) -> Result<JsValue, JsValue> {
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        with_js_error!(self
            .0
            .get_binary_properties(doc_type)
            .map_err(from_dpp_err)?
            .serialize(&serializer))
    }

    #[wasm_bindgen(js_name=getMetadata)]
    pub fn get_metadata(&self) -> Option<MetadataWasm> {
        self.0.metadata.clone().map(Into::into)
    }

    #[wasm_bindgen(js_name=setMetadata)]
    pub fn set_metadata(&mut self, metadata: MetadataWasm) {
        self.0.metadata = Some(metadata.into());
    }

    #[wasm_bindgen(js_name=toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsValue> {
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        with_js_error!(self.0.serialize(&serializer))
    }

    #[wasm_bindgen(js_name=toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        self.to_object()
    }

    #[wasm_bindgen(js_name=toBuffer)]
    pub fn to_buffer(&self) -> Result<Buffer, JsValue> {
        let bytes = self.0.to_buffer().map_err(from_dpp_err)?;
        Ok(Buffer::from_bytes(&bytes))
    }

    #[wasm_bindgen(js_name=hash)]
    pub fn hash(&self) -> Result<Vec<u8>, JsValue> {
        self.0.hash().map_err(from_dpp_err)
    }

    #[wasm_bindgen(js_name=from)]
    pub fn from(v: JsValue) -> Result<DataContractWasm, JsValue> {
        let json_contract: Value = with_js_error!(serde_wasm_bindgen::from_value(v))?;
        Ok(DataContract::try_from(json_contract)
            .map_err(from_dpp_err)?
            .into())
    }
}

#[wasm_bindgen(js_name=DataContractDefaults)]
pub struct DataContractDefaults;

#[wasm_bindgen(js_class=DataContractDefaults)]
impl DataContractDefaults {
    #[wasm_bindgen(getter = SCHEMA)]
    pub fn get_default_schema() -> String {
        SCHEMA_URI.to_string()
    }
}

impl DataContractWasm {
    pub fn inner(&self) -> &DataContract {
        &self.0
    }
}
