#![allow(clippy::from_over_into)]

use std::collections::BTreeMap;
use std::convert::TryFrom;

pub use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use wasm_bindgen::prelude::*;

use dpp::data_contract::{CreatedDataContract, DataContract, DATA_CONTRACT_SCHEMA_URI_V0};
use dpp::platform_value::string_encoding::Encoding;
use dpp::platform_value::{Bytes32, Value};

use dpp::serialization::PlatformSerializable;
use dpp::{platform_value,  ProtocolError};

use crate::identifier::identifier_from_js_value;
use crate::metadata::MetadataWasm;
use crate::utils::{Inner, IntoWasm, WithJsError};
use crate::{bail_js, with_js_error};
use crate::{buffer::Buffer, identifier::IdentifierWrapper};

#[wasm_bindgen(js_name=DataContract)]
#[derive(Debug, Clone)]
pub struct DataContractWasm {
    inner: DataContract,
    entropy_used: Option<Vec<u8>>,
}

/// CreatedDataContract contains entropy and is used to create
/// DataContractCreateTransition
impl std::convert::From<CreatedDataContract> for DataContractWasm {
    fn from(v: CreatedDataContract) -> Self {
        DataContractWasm {
            inner: v.data_contract,
            entropy_used: Some(v.entropy_used.to_vec()),
        }
    }
}

/// Regular DataContract does not contain entropy and is used
/// in DataContractUpdateTransition
impl std::convert::From<DataContract> for DataContractWasm {
    fn from(v: DataContract) -> Self {
        DataContractWasm {
            inner: v,
            entropy_used: None,
        }
    }
}

impl std::convert::From<&DataContractWasm> for DataContract {
    fn from(v: &DataContractWasm) -> Self {
        v.inner.clone()
    }
}

impl std::convert::TryFrom<&DataContractWasm> for CreatedDataContract {
    type Error = ProtocolError;
    fn try_from(v: &DataContractWasm) -> Result<Self, Self::Error> {
        Ok(Self {
            data_contract: v.to_owned().into(),
            entropy_used: if let Some(entropy_used) = &v.entropy_used {
                Bytes32::from_vec(entropy_used.to_owned())?
            } else {
                Bytes32::default()
            },
        })
    }
}

impl std::convert::Into<DataContract> for DataContractWasm {
    fn into(self) -> DataContract {
        self.inner
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

        DataContract::from_value(
            platform_value::to_value(parameters).expect("Implements Serialize"),
        )
        .with_js_error()
        .map(Into::into)
    }

    #[wasm_bindgen(js_name=getProtocolVersion)]
    pub fn get_protocol_version(&self) -> u32 {
        self.0.data_contract_protocol_version
    }

    #[wasm_bindgen(js_name=getId)]
    pub fn get_id(&self) -> IdentifierWrapper {
        self.inner.id.into()
    }

    #[wasm_bindgen(js_name=setId)]
    pub fn set_id(&mut self, id: &JsValue) -> Result<(), JsValue> {
        let id = identifier_from_js_value(id)?;
        self.inner.id = id;
        Ok(())
    }

    #[wasm_bindgen(js_name=getOwnerId)]
    pub fn get_owner_id(&self) -> IdentifierWrapper {
        self.inner.owner_id.into()
    }

    #[wasm_bindgen(js_name=getVersion)]
    pub fn get_version(&self) -> u32 {
        self.inner.version
    }

    #[wasm_bindgen(js_name=setVersion)]
    pub fn set_version(&mut self, v: u32) {
        self.inner.version = v;
    }

    #[wasm_bindgen(js_name=incrementVersion)]
    pub fn increment_version(&mut self) {
        self.inner.increment_version()
    }

    #[wasm_bindgen(js_name=getJsonSchemaId)]
    pub fn get_json_schema_id(&self) -> String {
        self.inner.id.to_string(Encoding::Base58)
    }

    #[wasm_bindgen(js_name=setJsonMetaSchema)]
    pub fn set_json_meta_schema(&mut self, schema: String) {
        self.inner.schema = schema;
    }

    #[wasm_bindgen(js_name=getJsonMetaSchema)]
    pub fn get_json_meta_schema(&self) -> String {
        self.inner.schema.clone()
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
            self.inner.documents = docs;
        } else {
            bail_js!("the parameter 'documents' is not an JS object")
        }
        Ok(())
    }

    #[wasm_bindgen(js_name=getDocuments)]
    pub fn get_documents(&self) -> Result<JsValue, JsValue> {
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        with_js_error!(self.inner.documents.serialize(&serializer))
    }

    #[wasm_bindgen(js_name=isDocumentDefined)]
    pub fn is_document_defined(&self, doc_type: String) -> bool {
        self.inner.is_document_defined(&doc_type)
    }

    #[wasm_bindgen(js_name=setDocumentSchema)]
    pub fn set_document_schema(
        &mut self,
        doc_type: String,
        schema: JsValue,
    ) -> Result<(), JsValue> {
        let json_schema: JsonValue = with_js_error!(serde_wasm_bindgen::from_value(schema))?;
        self.inner
            .set_document_json_schema(doc_type, json_schema)
            .with_js_error()?;
        Ok(())
    }

    #[wasm_bindgen(js_name=getDocumentSchema)]
    pub fn get_document_schema(&mut self, doc_type: &str) -> Result<JsValue, JsValue> {
        let doc_schema = self.inner.document_json_schema(doc_type).with_js_error()?;
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        with_js_error!(doc_schema.serialize(&serializer))
    }

    #[wasm_bindgen(js_name=getDocumentSchemaRef)]
    pub fn get_document_schema_ref(&self, doc_type: &str) -> Result<JsValue, JsValue> {
        with_js_error!(serde_wasm_bindgen::to_value(
            &self
                .inner
                .document_json_schema_ref(doc_type)
                .with_js_error()?
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
            self.inner.defs = Some(definitions);
        } else {
            bail_js!("the parameter 'definitions' is not an JS object");
        }
        Ok(())
    }

    #[wasm_bindgen(js_name=getDefinitions)]
    pub fn get_definitions(&self) -> Result<JsValue, JsValue> {
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        with_js_error!(self.inner.defs.serialize(&serializer))
    }

    #[wasm_bindgen(js_name=setEntropy)]
    pub fn set_entropy(&mut self, e: Vec<u8>) -> Result<(), JsValue> {
        self.entropy_used = Some(e);
        Ok(())
    }

    #[wasm_bindgen(js_name=getEntropy)]
    pub fn get_entropy(&mut self) -> JsValue {
        self.entropy_used
            .as_ref()
            .map(|e| Buffer::from_bytes(e.as_slice()).into())
            .unwrap_or(JsValue::undefined())
    }

    #[wasm_bindgen(js_name=getBinaryProperties)]
    pub fn get_binary_properties(&self, doc_type: &str) -> Result<JsValue, JsValue> {
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        with_js_error!(self
            .inner
            .get_binary_properties(doc_type)
            .with_js_error()?
            .serialize(&serializer))
    }

    #[wasm_bindgen(js_name=getMetadata)]
    pub fn get_metadata(&self) -> Option<MetadataWasm> {
        self.inner.metadata.clone().map(Into::into)
    }

    #[wasm_bindgen(js_name=setMetadata)]
    pub fn set_metadata(&mut self, metadata: JsValue) -> Result<(), JsValue> {
        self.inner.metadata = if !metadata.is_falsy() {
            let metadata = metadata.to_wasm::<MetadataWasm>("Metadata")?;
            Some(metadata.to_owned().into())
        } else {
            None
        };

        Ok(())
    }

    #[wasm_bindgen(js_name=toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsValue> {
        let value = self.inner.to_cleaned_object().with_js_error()?;
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        let object = with_js_error!(value.serialize(&serializer))?;

        js_sys::Reflect::set(
            &object,
            &Into::<JsValue>::into("$id".to_owned()),
            &Into::<JsValue>::into(Buffer::from_bytes_owned(self.inner.id.to_vec())),
        )
        .expect("target is an object");
        js_sys::Reflect::set(
            &object,
            &Into::<JsValue>::into("ownerId".to_owned()),
            &Into::<JsValue>::into(Buffer::from_bytes_owned(self.inner.owner_id.to_vec())),
        )
        .expect("target is an object");
        Ok(object)
    }

    #[wasm_bindgen(js_name=getConfig)]
    pub fn config(&self) -> Result<JsValue, JsValue> {
        Ok(serde_wasm_bindgen::to_value(&self.inner.config)?)
    }

    #[wasm_bindgen(js_name=setConfig)]
    pub fn set_config(&mut self, config: JsValue) -> Result<(), JsValue> {
        let res = serde_wasm_bindgen::from_value(config);
        self.inner.config = res.unwrap();
        Ok(())
    }

    #[wasm_bindgen(js_name=toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        let json = self.inner.to_json().with_js_error()?;
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        with_js_error!(json.serialize(&serializer))
    }

    #[wasm_bindgen(js_name=toBuffer)]
    pub fn to_buffer(&self) -> Result<Buffer, JsValue> {
        let bytes = PlatformSerializable::serialize(&self.inner).with_js_error()?;
        Ok(Buffer::from_bytes(&bytes))
    }

    #[wasm_bindgen(js_name=hash)]
    pub fn hash(&self) -> Result<Vec<u8>, JsValue> {
        self.inner.hash().with_js_error()
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

impl Inner for DataContractWasm {
    type InnerItem = DataContract;

    fn into_inner(self) -> DataContract {
        self.inner
    }

    fn inner(&self) -> &DataContract {
        &self.inner
    }

    fn inner_mut(&mut self) -> &mut DataContract {
        &mut self.inner
    }
}

impl DataContractWasm {
    pub fn inner(&self) -> &DataContract {
        &self.inner
    }
}

#[wasm_bindgen(js_name=DataContractDefaults)]
pub struct DataContractDefaults;

#[wasm_bindgen(js_class=DataContractDefaults)]
impl DataContractDefaults {
    #[wasm_bindgen(getter = SCHEMA)]
    pub fn get_default_schema() -> String {
        DATA_CONTRACT_SCHEMA_URI_V0.to_string()
    }
}
