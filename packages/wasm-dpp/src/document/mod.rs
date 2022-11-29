use dpp::prelude::Identifier;
use dpp::util::json_schema::JsonSchemaExt;
use dpp::util::json_value::{JsonValueExt, ReplaceWith};
use dpp::util::string_encoding::Encoding;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::convert::TryInto;
use wasm_bindgen::prelude::*;

use dpp::document::{Document, IDENTIFIER_FIELDS};

use crate::buffer::Buffer;
use crate::errors::{from_dpp_err, RustConversionError};
use crate::identifier::IdentifierWrapper;
use crate::lodash::lodash_set;
use crate::utils::to_serde_json_value;
use crate::with_js_error;
use crate::{DataContractWasm, MetadataWasm};

pub mod errors;

pub(super) enum BinaryType {
    Identifier,
    Buffer,
    None,
}

#[wasm_bindgen(js_name=Document)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentWasm(Document);

#[wasm_bindgen(js_class=Document)]
impl DocumentWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(
        js_raw_document: JsValue,
        js_data_contract: &DataContractWasm,
    ) -> Result<DocumentWasm, JsValue> {
        let mut raw_document = to_serde_json_value(&js_raw_document)?;

        raw_document
            .replace_identifier_paths(IDENTIFIER_FIELDS, ReplaceWith::Bytes)
            .unwrap();

        // TODO dynamic fields must be replaced as well??

        let document =
            Document::from_raw_document(raw_document, js_data_contract.to_owned().into())
                .map_err(from_dpp_err)
                .unwrap();

        Ok(document.into())
    }

    #[wasm_bindgen(js_name=getProtocolVersion)]
    pub fn get_protocol_version(&self) -> u32 {
        self.0.protocol_version
    }

    #[wasm_bindgen(js_name=getId)]
    pub fn get_id(&self) -> IdentifierWrapper {
        self.0.id.clone().into()
    }

    #[wasm_bindgen(js_name=setId)]
    pub fn set_id(&mut self, js_id: IdentifierWrapper) {
        self.0.id = js_id.inner();
    }

    #[wasm_bindgen(js_name=getType)]
    pub fn get_type(&self) -> String {
        self.0.document_type.clone()
    }

    #[wasm_bindgen(js_name=getDataContractId)]
    pub fn get_data_contract_id(&self) -> IdentifierWrapper {
        self.0.data_contract_id.clone().into()
    }

    #[wasm_bindgen(js_name=getDataContract)]
    pub fn get_data_contract(&self) -> DataContractWasm {
        self.0.data_contract.clone().into()
    }

    #[wasm_bindgen(js_name=getOwnerId)]
    pub fn get_owner_id(&self) -> IdentifierWrapper {
        self.0.owner_id.clone().into()
    }

    #[wasm_bindgen(js_name=setRevision)]
    pub fn set_revision(&mut self, rev: u32) {
        self.0.revision = rev
    }

    #[wasm_bindgen(js_name=getRevision)]
    pub fn get_revision(&self) -> u32 {
        self.0.revision
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

    #[wasm_bindgen(js_name=setData)]
    pub fn set_data(&mut self, d: JsValue) -> Result<(), JsValue> {
        self.0.data = with_js_error!(serde_wasm_bindgen::from_value(d))?;
        Ok(())
    }

    #[wasm_bindgen(js_name=getData)]
    pub fn get_data(&mut self) -> Result<JsValue, JsValue> {
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        Ok(with_js_error!(self.0.get_data().serialize(&serializer))?)
    }

    #[wasm_bindgen(js_name=set)]
    pub fn set(&mut self, path: String, js_value: JsValue) -> Result<(), JsValue> {
        let value = to_serde_json_value(&js_value)?;
        self.0.set(&path, value).map_err(from_dpp_err)?;

        Ok(())
    }

    #[wasm_bindgen(js_name=get)]
    pub fn get(&mut self, path: String) -> JsValue {
        let binary_type = self.get_binary_type(&path);

        if let Some(value) = self.0.get(&path) {
            match binary_type {
                BinaryType::Identifier => {
                    if let Value::String(id_string) = value {
                        let id: IdentifierWrapper =
                            Identifier::from_string(id_string, Encoding::Base58)
                                .unwrap()
                                .into();
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

    #[wasm_bindgen(js_name=setCreatedAt)]
    pub fn set_created_at(&mut self, ts: f64) {
        self.0.created_at = Some(ts as i64);
    }

    #[wasm_bindgen(js_name=setUpdatedAt)]
    pub fn set_updated_at(&mut self, ts: f64) {
        self.0.updated_at = Some(ts as i64);
    }

    #[wasm_bindgen(js_name=getCreatedAt)]
    pub fn get_created_at(&self) -> Option<f64> {
        self.0.created_at.map(|v| v as f64)
    }

    #[wasm_bindgen(js_name=getUpdatedAt)]
    pub fn get_updated_at(&self) -> Option<f64> {
        self.0.updated_at.map(|v| v as f64)
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
        let mut value = self.0.to_object(false).map_err(from_dpp_err)?;

        let (identifiers_paths, binary_paths) = self.0.get_binary_and_identifier_paths();
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        let js_value = value.serialize(&serializer)?;

        for path in identifiers_paths.into_iter().chain(IDENTIFIER_FIELDS) {
            if let Ok(bytes) = value.remove_path_into::<Vec<u8>>(path) {
                let id = IdentifierWrapper::new(bytes)?;
                lodash_set(&js_value, path, id.into());
            }
        }

        for path in binary_paths {
            if let Ok(bytes) = value.remove_path_into::<Vec<u8>>(path) {
                let buffer = Buffer::from_bytes(&bytes);
                lodash_set(&js_value, path, buffer.into());
            }
        }

        Ok(js_value)
    }

    #[wasm_bindgen(js_name=toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        let value = self.0.to_json().map_err(from_dpp_err)?;
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();

        with_js_error!(value.serialize(&serializer))
    }

    #[wasm_bindgen(js_name=toBuffer)]
    pub fn to_buffer(&self) -> Result<Buffer, JsValue> {
        let bytes = self.0.to_buffer().map_err(from_dpp_err)?;

        Ok(Buffer::from_bytes(&bytes))
    }

    #[wasm_bindgen(js_name=hash)]
    pub fn hash(&self) -> Result<Buffer, JsValue> {
        let bytes = self.0.hash().map_err(from_dpp_err)?;
        Ok(Buffer::from_bytes(&bytes))
    }
}

impl DocumentWasm {
    fn get_binary_type(&self, path: &String) -> BinaryType {
        let maybe_binary_properties = self
            .0
            .data_contract
            .get_binary_properties(&self.0.document_type);

        if let Ok(binary_properties) = maybe_binary_properties {
            if let Some(data) = binary_properties.get(path) {
                if data.is_type_of_identifier() {
                    return BinaryType::Identifier;
                }
                return BinaryType::Buffer;
            }
        }
        BinaryType::None
    }
}

impl From<Document> for DocumentWasm {
    fn from(d: Document) -> Self {
        DocumentWasm(d)
    }
}
