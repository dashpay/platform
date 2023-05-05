#![allow(clippy::from_over_into)]

use std::collections::BTreeMap;
use std::convert::{TryFrom, TryInto};

pub use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use wasm_bindgen::prelude::*;

use dpp::data_contract::{DataContract, SCHEMA_URI};
use dpp::platform_value::string_encoding::Encoding;
use dpp::platform_value::{Bytes32, Value};

use dpp::serialization_traits::PlatformSerializable;
use dpp::{platform_value, Convertible};

use crate::errors::RustConversionError;
use crate::identifier::identifier_from_js_value;
use crate::metadata::MetadataWasm;
use crate::utils::{IntoWasm, WithJsError};
use crate::{bail_js, with_js_error};
use crate::{buffer::Buffer, identifier::IdentifierWrapper};

#[wasm_bindgen(js_name=DataContract)]
#[derive(Debug, Clone)]
pub struct DataContractWasm(pub(crate) DataContract);

impl std::convert::From<DataContract> for DataContractWasm {
    fn from(v: DataContract) -> Self {
        DataContractWasm(v)
    }
}

impl std::convert::From<&DataContractWasm> for DataContract {
    fn from(v: &DataContractWasm) -> Self {
        v.0.clone()
    }
}

impl std::convert::Into<DataContract> for DataContractWasm {
    fn into(self) -> DataContract {
        self.0
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DataContractParameters {
    #[serde(
        rename = "$schema",
        skip_serializing_if = "serde_json::Value::is_null",
        default
    )]
    schema: serde_json::Value,
    #[serde(rename = "$id", skip_serializing_if = "Option::is_none")]
    id: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    owner_id: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "serde_json::Value::is_null", default)]
    documents: serde_json::Value,
    #[serde(
        skip_serializing_if = "serde_json::Value::is_null",
        default,
        rename = "$defs"
    )]
    defs: serde_json::Value,
    #[serde(skip_serializing_if = "serde_json::Value::is_null", default)]
    protocol_version: serde_json::Value,
    #[serde(skip_serializing_if = "serde_json::Value::is_null", default)]
    version: serde_json::Value,

    #[serde(flatten)]
    _extras: serde_json::Value, // Captures excess fields to trigger validation failure later.
}

pub fn js_value_to_data_contract_value(object: JsValue) -> Result<Value, JsValue> {
    let parameters: DataContractParameters =
        with_js_error!(serde_wasm_bindgen::from_value(object))?;

    platform_value::to_value(parameters).map_err(|e| e.to_string().into())
}

#[wasm_bindgen(js_class=DataContract)]
impl DataContractWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(raw_parameters: JsValue) -> Result<DataContractWasm, JsValue> {
        let parameters: DataContractParameters =
            with_js_error!(serde_wasm_bindgen::from_value(raw_parameters))?;

        DataContract::from_raw_object(
            platform_value::to_value(parameters).expect("Implements Serialize"),
        )
        .with_js_error()
        .map(Into::into)
    }

    #[wasm_bindgen(js_name=getProtocolVersion)]
    pub fn get_protocol_version(&self) -> u32 {
        self.0.protocol_version
    }

    #[wasm_bindgen(js_name=getId)]
    pub fn get_id(&self) -> IdentifierWrapper {
        self.0.id.into()
    }

    #[wasm_bindgen(js_name=setId)]
    pub fn set_id(&mut self, id: &JsValue) -> Result<(), JsValue> {
        let id = identifier_from_js_value(id)?;
        self.0.id = id;
        Ok(())
    }

    #[wasm_bindgen(js_name=getOwnerId)]
    pub fn get_owner_id(&self) -> IdentifierWrapper {
        self.0.owner_id.into()
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
        let json_value: JsonValue = with_js_error!(serde_wasm_bindgen::from_value(documents))?;

        let mut docs: BTreeMap<String, JsonValue> = BTreeMap::new();
        if let JsonValue::Object(o) = json_value {
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
        let json_schema: JsonValue = with_js_error!(serde_wasm_bindgen::from_value(schema))?;
        self.0
            .set_document_schema(doc_type, json_schema)
            .with_js_error()?;
        Ok(())
    }

    #[wasm_bindgen(js_name=getDocumentSchema)]
    pub fn get_document_schema(&mut self, doc_type: &str) -> Result<JsValue, JsValue> {
        let doc_schema = self.0.get_document_schema(doc_type).with_js_error()?;
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        with_js_error!(doc_schema.serialize(&serializer))
    }

    #[wasm_bindgen(js_name=getDocumentSchemaRef)]
    pub fn get_document_schema_ref(&self, doc_type: &str) -> Result<JsValue, JsValue> {
        with_js_error!(serde_wasm_bindgen::to_value(
            &self.0.get_document_schema_ref(doc_type).with_js_error()?
        ))
    }

    #[wasm_bindgen(js_name=setDefinitions)]
    pub fn set_definitions(&mut self, definitions: JsValue) -> Result<(), JsValue> {
        let json_value: JsonValue = with_js_error!(serde_wasm_bindgen::from_value(definitions))?;
        let mut definitions: BTreeMap<String, JsonValue> = BTreeMap::new();
        if let JsonValue::Object(o) = json_value {
            for (k, v) in o.into_iter() {
                // v must be a Object
                if !v.is_object() {
                    bail_js!("{:?} is not an object", v);
                }
                definitions.insert(k, v);
            }
            if definitions.is_empty() {
                bail_js!("`definitions` cannot be empty");
            }
            self.0.defs = Some(definitions);
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
        self.0.entropy = Bytes32::new(entropy);
        Ok(())
    }

    #[wasm_bindgen(js_name=getEntropy)]
    pub fn get_entropy(&mut self) -> Buffer {
        Buffer::from_bytes_owned(self.0.entropy.to_vec())
    }

    #[wasm_bindgen(js_name=getBinaryProperties)]
    pub fn get_binary_properties(&self, doc_type: &str) -> Result<JsValue, JsValue> {
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        with_js_error!(self
            .0
            .get_binary_properties(doc_type)
            .with_js_error()?
            .serialize(&serializer))
    }

    #[wasm_bindgen(js_name=getMetadata)]
    pub fn get_metadata(&self) -> Option<MetadataWasm> {
        self.0.metadata.clone().map(Into::into)
    }

    #[wasm_bindgen(js_name=setMetadata)]
    pub fn set_metadata(&mut self, metadata: JsValue) -> Result<(), JsValue> {
        self.0.metadata = if !metadata.is_falsy() {
            let metadata = metadata.to_wasm::<MetadataWasm>("Metadata")?;
            Some(metadata.to_owned().into())
        } else {
            None
        };

        Ok(())
    }

    #[wasm_bindgen(js_name=toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsValue> {
        let value = self.0.to_cleaned_object().with_js_error()?;
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        let object = with_js_error!(value.serialize(&serializer))?;

        js_sys::Reflect::set(
            &object,
            &Into::<JsValue>::into("$id".to_owned()),
            &Into::<JsValue>::into(Buffer::from_bytes_owned(self.0.id.to_vec())),
        )
        .expect("target is an object");
        js_sys::Reflect::set(
            &object,
            &Into::<JsValue>::into("ownerId".to_owned()),
            &Into::<JsValue>::into(Buffer::from_bytes_owned(self.0.owner_id.to_vec())),
        )
        .expect("target is an object");
        Ok(object)
    }

    #[wasm_bindgen(js_name=toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        let json = self.0.to_json().with_js_error()?;
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        with_js_error!(json.serialize(&serializer))
    }

    #[wasm_bindgen(js_name=toBuffer)]
    pub fn to_buffer(&self) -> Result<Buffer, JsValue> {
        let bytes = PlatformSerializable::serialize(&self.0).with_js_error()?;
        Ok(Buffer::from_bytes(&bytes))
    }

    #[wasm_bindgen(js_name=hash)]
    pub fn hash(&self) -> Result<Vec<u8>, JsValue> {
        self.0.hash().with_js_error()
    }

    #[wasm_bindgen(js_name=from)]
    pub fn from_js_value(v: JsValue) -> Result<DataContractWasm, JsValue> {
        let json_contract: JsonValue = with_js_error!(serde_wasm_bindgen::from_value(v))?;
        Ok(DataContract::try_from(json_contract)
            .with_js_error()?
            .into())
    }

    #[wasm_bindgen(js_name=fromBuffer)]
    pub fn from_buffer(b: &[u8]) -> Result<DataContractWasm, JsValue> {
        let data_contract = DataContract::from_cbor(b).with_js_error()?;
        Ok(data_contract.into())
    }

    #[wasm_bindgen(js_name=clone)]
    pub fn deep_clone(&self) -> Self {
        self.clone()
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

#[test]
fn test_query_many() {}
