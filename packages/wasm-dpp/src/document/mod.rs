use dpp::dashcore::anyhow::Context;
use dpp::prelude::{DataContract, Identifier};
use dpp::util::json_schema::JsonSchemaExt;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use wasm_bindgen::prelude::*;

use crate::buffer::Buffer;

use crate::identifier::IdentifierWrapper;
use crate::lodash::lodash_set;
use crate::utils::{try_to_u64, WithJsError};
use crate::utils::{with_serde_to_json_value, ToSerdeJSONExt};
use crate::with_js_error;
use crate::DataContractWasm;

pub mod errors;
pub use state_transition::*;
pub mod document_facade;
mod extended_document;
mod factory;
pub mod fetch_and_validate_data_contract;
pub mod generate_document_id;
pub mod state_transition;
mod validator;

pub use document_batch_transition::DocumentsBatchTransitionWasm;

use dpp::document::{Document, EXTENDED_DOCUMENT_IDENTIFIER_FIELDS, IDENTIFIER_FIELDS};

pub use extended_document::ExtendedDocumentWasm;

use dpp::document::extended_document::property_names;
use dpp::platform_value::btreemap_extensions::BTreeValueMapReplacementPathHelper;
use dpp::platform_value::converter::serde_json::BTreeValueJsonConverter;
use dpp::platform_value::ReplacementType;
use dpp::platform_value::Value;
use dpp::{platform_value, ProtocolError};
pub use factory::DocumentFactoryWASM;
use serde_json::Value as JsonValue;
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
        js_document_type_name: JsValue,
    ) -> Result<DocumentWasm, JsValue> {
        let mut raw_document: Value = with_serde_to_json_value(&js_raw_document)?.into();

        let document_type_name = js_document_type_name
            .as_string()
            .ok_or(anyhow!(
                "expected a string for the document type, got {:?}",
                js_document_type_name
            ))
            .with_js_error()?;

        let (identifier_paths, _) = js_data_contract
            .inner()
            .get_identifiers_and_binary_paths(document_type_name.as_str())
            .with_js_error()?;

        // TODO: figure out a better way to replace identifiers
        raw_document
            .replace_at_paths(
                identifier_paths
                    .into_iter()
                    .chain(EXTENDED_DOCUMENT_IDENTIFIER_FIELDS),
                ReplacementType::Identifier,
            )
            .map_err(ProtocolError::ValueError)
            .with_js_error()?;
        // The binary paths are not being converted, because they always should be a `Buffer`. `Buffer` is always an Array

        let document = Document::from_platform_value(raw_document).with_js_error()?;

        Ok(document.into())
    }

    #[wasm_bindgen(js_name=getId)]
    pub fn get_id(&self) -> IdentifierWrapper {
        self.0.id.into()
    }

    #[wasm_bindgen(js_name=setId)]
    pub fn set_id(&mut self, js_id: IdentifierWrapper) {
        self.0.id = js_id.into();
    }

    #[wasm_bindgen(js_name=setOwnerId)]
    pub fn set_owner_id(&mut self, owner_id: IdentifierWrapper) {
        self.0.owner_id = owner_id.into();
    }

    #[wasm_bindgen(js_name=getOwnerId)]
    pub fn get_owner_id(&self) -> IdentifierWrapper {
        self.0.owner_id.into()
    }

    #[wasm_bindgen(js_name=setRevision)]
    pub fn set_revision(&mut self, revision: Option<u32>) {
        // TODO: JS feeding Number here (u32). Is it okay to cast u32 to u64?
        self.0.revision = revision.map(|r| r as u64);
    }

    #[wasm_bindgen(js_name=getRevision)]
    pub fn get_revision(&self) -> Option<u32> {
        // TODO: JS tests expecting Number (u32). Is it okay to cast u64 to u32 here?
        self.0.revision.map(|r| r as u32)
    }

    #[wasm_bindgen(js_name=setData)]
    pub fn set_data(&mut self, d: JsValue) -> Result<(), JsValue> {
        let properties_as_value = d.with_serde_to_platform_value()?;
        self.0.properties = properties_as_value
            .into_btree_string_map()
            .map_err(ProtocolError::ValueError)
            .with_js_error()?;
        Ok(())
    }

    #[wasm_bindgen(js_name=getData)]
    pub fn get_data(&mut self) -> Result<JsValue, JsValue> {
        let json_value: JsonValue = self
            .0
            .properties
            .to_json_value()
            .map_err(ProtocolError::ValueError)
            .with_js_error()?;

        let js_value = json_value.serialize(&serde_wasm_bindgen::Serializer::json_compatible())?;
        Ok(js_value)
    }

    #[wasm_bindgen(js_name=set)]
    pub fn set(&mut self, path: String, js_value_to_set: JsValue) -> Result<(), JsValue> {
        let value = js_value_to_set.with_serde_to_platform_value()?;
        self.0.set(&path, value);
        Ok(())
    }

    #[wasm_bindgen(js_name=get)]
    pub fn get(&mut self, path: String) -> Result<JsValue, JsValue> {
        if let Some(value) = self.0.get(&path) {
            match value {
                Value::Bytes(bytes) => {
                    return Ok(Buffer::from_bytes(bytes.as_slice()).into());
                }
                Value::Bytes20(bytes) => {
                    return Ok(Buffer::from_bytes(bytes.as_slice()).into());
                }
                Value::Bytes32(bytes) => {
                    return Ok(Buffer::from_bytes(bytes.as_slice()).into());
                }
                Value::Bytes36(bytes) => {
                    return Ok(Buffer::from_bytes(bytes.as_slice()).into());
                }
                Value::Identifier(identifier) => {
                    let id: IdentifierWrapper = Identifier::new(*identifier).into();
                    return Ok(id.into());
                }
                _ => {
                    let json_value_result: Result<JsonValue, ProtocolError> =
                        value.clone().try_into().map_err(ProtocolError::ValueError);
                    let json_value = json_value_result.with_js_error()?;
                    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
                    if let Ok(js_value) = json_value.serialize(&serializer) {
                        return Ok(js_value);
                    }
                }
            }
        }

        Ok(JsValue::undefined())
    }

    #[wasm_bindgen(js_name=setCreatedAt)]
    pub fn set_created_at(&mut self, number: JsValue) -> Result<(), JsValue> {
        let ts = try_to_u64(number)
            .context("setting createdAt in Document")
            .with_js_error()?;

        self.0.created_at = Some(ts);
        Ok(())
    }

    #[wasm_bindgen(js_name=setUpdatedAt)]
    pub fn set_updated_at(&mut self, number: JsValue) -> Result<(), JsValue> {
        let ts = try_to_u64(number)
            .context("setting updatedAt in Document")
            .with_js_error()?;
        self.0.updated_at = Some(ts);
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

    #[wasm_bindgen(js_name=toObject)]
    pub fn to_object(
        &self,
        options: &JsValue,
        data_contract: &DataContractWasm,
        document_type_name: &str,
    ) -> Result<JsValue, JsValue> {
        let options: ConversionOptions = if !options.is_undefined() && options.is_object() {
            let raw_options = options.with_serde_to_platform_value()?;
            platform_value::from_value(raw_options)
                .map_err(ProtocolError::ValueError)
                .with_js_error()?
        } else {
            Default::default()
        };
        let mut value = self.0.to_object().with_js_error()?;

        let (identifiers_paths, binary_paths) =
            Document::get_identifiers_and_binary_paths(&data_contract.0, document_type_name)
                .with_js_error()?;
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        let js_value = value.serialize(&serializer)?;

        for path in identifiers_paths.into_iter().chain(IDENTIFIER_FIELDS) {
            if let Ok(bytes) = value.remove_value_at_path_as_bytes(path) {
                let buffer = Buffer::from_bytes_owned(bytes);
                if !options.skip_identifiers_conversion {
                    lodash_set(&js_value, path, buffer.into());
                } else {
                    let id = IdentifierWrapper::new(buffer.into());
                    lodash_set(&js_value, path, id.into());
                }
            }
        }

        for path in binary_paths {
            if let Ok(bytes) = value.remove_value_at_path_as_bytes(path) {
                let buffer = Buffer::from_bytes_owned(bytes);
                lodash_set(&js_value, path, buffer.into());
            }
        }

        Ok(js_value)
    }

    #[wasm_bindgen(js_name=toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        let value = self.0.to_cbor().with_js_error()?;
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();

        with_js_error!(value.serialize(&serializer))
    }

    #[wasm_bindgen(js_name=toBuffer)]
    pub fn to_buffer(&self) -> Result<Buffer, JsValue> {
        let bytes = self.0.to_cbor().with_js_error()?;

        Ok(Buffer::from_bytes(&bytes))
    }

    #[wasm_bindgen(js_name=hash)]
    pub fn hash(
        &self,
        data_contract: DataContractWasm,
        document_type_name: String,
    ) -> Result<Buffer, JsValue> {
        let document_type = data_contract
            .0
            .document_type_for_name(document_type_name.as_str())
            .with_js_error()?;
        let bytes = self
            .0
            .hash(&data_contract.0, document_type)
            .with_js_error()?;
        Ok(Buffer::from_bytes(&bytes))
    }

    #[wasm_bindgen(js_name=clone)]
    pub fn deep_clone(&self) -> DocumentWasm {
        self.clone()
    }
}

impl DocumentWasm {
    fn get_binary_type_of_path(
        &self,
        path: &String,
        data_contract: &DataContractWasm,
        document_type_name: String,
    ) -> BinaryType {
        let maybe_binary_properties = data_contract
            .0
            .get_binary_properties(document_type_name.as_str());

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
pub(crate) fn document_data_to_bytes(
    document: &mut Document,
    contract: &DataContract,
    document_type: &str,
) -> Result<(), JsValue> {
    let (identifier_paths, binary_paths): (Vec<_>, Vec<_>) = contract
        .get_identifiers_and_binary_paths_owned(document_type)
        .with_js_error()?;
    document
        .properties
        .replace_at_paths(identifier_paths, ReplacementType::Identifier)
        .map_err(ProtocolError::ValueError)
        .with_js_error()?;
    document
        .properties
        .replace_at_paths(binary_paths, ReplacementType::BinaryBytes)
        .map_err(ProtocolError::ValueError)
        .with_js_error()?;
    Ok(())
}

pub(crate) fn raw_document_from_js_value(
    js_raw_document: &JsValue,
    data_contract: &DataContract,
) -> Result<Value, JsValue> {
    let mut raw_document = js_raw_document.with_serde_to_platform_value()?;

    let document_type_name = raw_document
        .get_str(property_names::DOCUMENT_TYPE)
        .map_err(ProtocolError::ValueError)
        .with_js_error()?;

    let (identifier_paths, binary_paths) = data_contract
        .get_identifiers_and_binary_paths(document_type_name)
        .with_js_error()?;

    raw_document
        .replace_at_paths(identifier_paths, ReplacementType::Identifier)
        .map_err(ProtocolError::ValueError)
        .with_js_error()?;
    raw_document
        .replace_at_paths(binary_paths, ReplacementType::BinaryBytes)
        .map_err(ProtocolError::ValueError)
        .with_js_error()?;

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
