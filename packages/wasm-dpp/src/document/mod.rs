use dpp::dashcore::anyhow::Context;
use dpp::prelude::{DataContract, Identifier};
use dpp::util::json_schema::JsonSchemaExt;
use dpp::util::json_value::{JsonValueExt, ReplaceWith};
use dpp::util::string_encoding::Encoding;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::convert::{self, TryInto};
use wasm_bindgen::prelude::*;

use dpp::document::{property_names, Document, IDENTIFIER_FIELDS};

use crate::buffer::Buffer;
use crate::errors::RustConversionError;
use crate::identifier::{identifier_from_js_value, IdentifierWrapper};
use crate::lodash::lodash_set;
use crate::utils::{replace_identifiers_with_bytes_without_failing, ToSerdeJSONExt};
use crate::utils::{try_to_u64, WithJsError};
use crate::with_js_error;
use crate::{DataContractWasm, MetadataWasm};

pub mod errors;
pub use state_transition::*;
pub mod document_facade;
mod factory;
pub mod fetch_and_validate_data_contract;
pub mod state_transition;
mod validator;

pub use document_batch_transition::DocumentsBatchTransitionWASM;
pub use factory::DocumentFactoryWASM;
pub use validator::DocumentValidatorWasm;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConversionOptions {
    #[serde(default)]
    pub skip_identifiers_conversion: bool,
}

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
        let raw_document = raw_document_from_js_value(&js_raw_document, js_data_contract.inner())?;

        let document =
            Document::from_raw_document(raw_document, js_data_contract.to_owned().into())
                .with_js_error()?;

        Ok(document.into())
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
    pub fn set_id(&mut self, js_id: IdentifierWrapper) {
        self.0.id = js_id.inner();
    }

    #[wasm_bindgen(js_name=getType)]
    pub fn get_type(&self) -> String {
        self.0.document_type.clone()
    }

    #[wasm_bindgen(js_name=getDataContractId)]
    pub fn get_data_contract_id(&self) -> IdentifierWrapper {
        self.0.data_contract_id.into()
    }

    #[wasm_bindgen(js_name=setDataContractId)]
    pub fn set_data_contract_id(&mut self, js_data_contract_id: &JsValue) -> Result<(), JsValue> {
        let data_contract_id = identifier_from_js_value(js_data_contract_id)?;
        self.0.data_contract_id = data_contract_id;
        Ok(())
    }

    #[wasm_bindgen(js_name=getDataContract)]
    pub fn get_data_contract(&self) -> DataContractWasm {
        self.0.data_contract.clone().into()
    }

    #[wasm_bindgen(js_name=setOwnerId)]
    pub fn set_owner_id(&mut self, owner_id: IdentifierWrapper) {
        self.0.owner_id = owner_id.inner();
    }

    #[wasm_bindgen(js_name=getOwnerId)]
    pub fn get_owner_id(&self) -> IdentifierWrapper {
        self.0.owner_id.into()
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
    pub fn get_entropy(&mut self) -> Buffer {
        Buffer::from_bytes(&self.0.entropy)
    }

    #[wasm_bindgen(js_name=setData)]
    pub fn set_data(&mut self, document_data: JsValue) -> Result<(), JsValue> {
        self.0.data = document_data.with_serde_to_json_value()?;
        document_data_to_bytes(&mut self.0)
    }

    #[wasm_bindgen(js_name=getData)]
    pub fn get_data(&mut self) -> Result<JsValue, JsValue> {
        let js_value = self
            .0
            .data
            .serialize(&serde_wasm_bindgen::Serializer::json_compatible())?;

        let (identifier_paths, binary_paths) = self
            .0
            .data_contract
            .get_identifiers_and_binary_paths(&self.0.document_type)
            .with_js_error()?;

        for path in identifier_paths {
            if let Ok(value) = self.0.data.get_value(path) {
                let bytes: Vec<u8> = serde_json::from_value(value.to_owned()).with_js_error()?;
                let id = <IdentifierWrapper as convert::From<Identifier>>::from(
                    Identifier::from_bytes(&bytes).unwrap(),
                );
                lodash_set(&js_value, path, id.into());
            }
        }
        for path in binary_paths {
            if let Ok(value) = self.0.data.get_value(path) {
                let bytes: Vec<u8> = serde_json::from_value(value.to_owned()).with_js_error()?;
                let buffer = Buffer::from_bytes(&bytes);
                lodash_set(&js_value, path, buffer.into());
            }
        }

        Ok(js_value)
    }

    #[wasm_bindgen(js_name=set)]
    pub fn set(&mut self, path: String, js_value_to_set: JsValue) -> Result<(), JsValue> {
        let mut value_to_set = if js_value_to_set.is_null() || js_value_to_set.is_undefined() {
            serde_json::Value::Null
        } else {
            js_value_to_set.with_serde_to_json_value()?
        };

        let (identifier_paths, _) = self.0.get_identifiers_and_binary_paths().with_js_error()?;
        for property_path in identifier_paths {
            if property_path == path {
                let id_string = value_to_set
                    .as_str()
                    .context("the value must be a string")
                    .with_js_error()?;
                let id = Identifier::from_string(id_string, Encoding::Base58).with_js_error()?;
                let new_value = serde_json::to_value(id.as_bytes()).with_js_error()?;

                return self.0.set(&path, new_value).with_js_error();
            } else if property_path.starts_with(&path) {
                let (_, suffix) = property_path.split_at(path.len() + 1);

                if value_to_set.get_value(suffix).is_ok() {
                    let id_string = value_to_set
                        .remove_path_into::<String>(suffix)
                        .with_context(|| format!("unable convert `{path}` into string"))
                        .map_err(|e| format!("{e:#}"))?;
                    let id: IdentifierWrapper =
                        Identifier::from_string(&id_string, Encoding::Base58)
                            .with_js_error()?
                            .into();
                    let new_value = serde_json::to_value(id.inner().as_bytes()).with_js_error()?;
                    value_to_set
                        .insert_with_path(suffix, new_value)
                        .with_js_error()?;

                    return self.0.set(&path, value_to_set).with_js_error();
                }
            }
        }

        self.0.set(&path, value_to_set).with_js_error()
    }

    #[wasm_bindgen(js_name=get)]
    pub fn get(&mut self, path: String) -> JsValue {
        let binary_type = self.get_binary_type_of_path(&path);

        if let Some(value) = self.0.get(&path) {
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

    #[wasm_bindgen(js_name=setCreatedAt)]
    pub fn set_created_at(&mut self, number: JsValue) -> Result<(), JsValue> {
        let ts = try_to_u64(number)
            .context("setting createdAt in Document")
            .with_js_error()?;
        self.0.created_at = Some(ts as i64);
        Ok(())
    }

    #[wasm_bindgen(js_name=setUpdatedAt)]
    pub fn set_updated_at(&mut self, number: JsValue) -> Result<(), JsValue> {
        let ts = try_to_u64(number)
            .context("setting updatedAt in Document")
            .with_js_error()?;
        self.0.updated_at = Some(ts as i64);
        Ok(())
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
    pub fn set_metadata(&mut self, metadata: &MetadataWasm) {
        self.0.metadata = Some(metadata.into());
    }

    #[wasm_bindgen(js_name=toObject)]
    pub fn to_object(&self, options: &JsValue) -> Result<JsValue, JsValue> {
        let options: ConversionOptions = if !options.is_undefined() && options.is_object() {
            let raw_options = options.with_serde_to_json_value()?;
            serde_json::from_value(raw_options).with_js_error()?
        } else {
            Default::default()
        };
        let mut value = self.0.to_object().with_js_error()?;

        let (identifiers_paths, binary_paths) =
            self.0.get_identifiers_and_binary_paths().with_js_error()?;
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        let js_value = value.serialize(&serializer)?;

        for path in identifiers_paths.into_iter().chain(IDENTIFIER_FIELDS) {
            if let Ok(bytes) = value.remove_path_into::<Vec<u8>>(path) {
                if !options.skip_identifiers_conversion {
                    let buffer = Buffer::from_bytes(&bytes);
                    lodash_set(&js_value, path, buffer.into());
                } else {
                    let id = IdentifierWrapper::new(bytes)?;
                    lodash_set(&js_value, path, id.into());
                }
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
        let value = self.0.to_json().with_js_error()?;
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();

        with_js_error!(value.serialize(&serializer))
    }

    #[wasm_bindgen(js_name=toBuffer)]
    pub fn to_buffer(&self) -> Result<Buffer, JsValue> {
        let bytes = self.0.to_buffer().with_js_error()?;

        Ok(Buffer::from_bytes(&bytes))
    }

    #[wasm_bindgen(js_name=hash)]
    pub fn hash(&self) -> Result<Buffer, JsValue> {
        let bytes = self.0.hash().with_js_error()?;
        Ok(Buffer::from_bytes(&bytes))
    }

    #[wasm_bindgen(js_name=clone)]
    pub fn deep_clone(&self) -> DocumentWasm {
        self.clone()
    }
}

impl DocumentWasm {
    fn get_binary_type_of_path(&self, path: &String) -> BinaryType {
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

/// document's dynamic data, regardless they are identifiers or binary, they should
/// be stored as arrays of int
pub(crate) fn document_data_to_bytes(document: &mut Document) -> Result<(), JsValue> {
    let mut document_data = document.data.take();
    let (identifier_paths, binary_paths) = document
        .get_identifiers_and_binary_paths()
        .with_js_error()?;
    document_data
        .replace_identifier_paths(identifier_paths, ReplaceWith::Bytes)
        .with_js_error()?;
    document_data
        .replace_binary_paths(binary_paths, ReplaceWith::Bytes)
        .with_js_error()?;
    document.set_data(document_data);
    Ok(())
}

pub(crate) fn raw_document_from_js_value(
    js_raw_document: &JsValue,
    data_contract: &DataContract,
) -> Result<Value, JsValue> {
    let mut raw_document = js_raw_document.with_serde_to_json_value()?;

    let document_type = raw_document
        .get_string(property_names::DOCUMENT_TYPE)
        .with_js_error()?;

    let (identifier_paths, _) = data_contract
        .get_identifiers_and_binary_paths(document_type)
        .with_js_error()?;

    replace_identifiers_with_bytes_without_failing(
        &mut raw_document,
        identifier_paths.into_iter().chain(IDENTIFIER_FIELDS),
    );

    // The binary paths are not being converted, because they always should be a `Buffer`. `Buffer` is always an Array
    Ok(raw_document)
}

impl From<Document> for DocumentWasm {
    fn from(d: Document) -> Self {
        DocumentWasm(d)
    }
}

impl From<DocumentWasm> for Document {
    fn from(d: DocumentWasm) -> Self {
        d.0
    }
}

impl From<&DocumentWasm> for Document {
    fn from(d: &DocumentWasm) -> Self {
        d.0.clone()
    }
}
