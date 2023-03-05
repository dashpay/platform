use dpp::document::document_transition::document_base_transition::JsonValue;
use dpp::document::{
    extended_document_property_names, ExtendedDocument, EXTENDED_DOCUMENT_IDENTIFIER_FIELDS,
};

use dpp::platform_value::{ReplacementType, Value};
use dpp::prelude::{Identifier, Revision};
use dpp::util::json_schema::JsonSchemaExt;
use dpp::util::json_value::{JsonValueExt, ReplaceWith};

use dpp::ProtocolError;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use wasm_bindgen::prelude::*;

use crate::buffer::Buffer;
use crate::document::BinaryType;
use crate::errors::RustConversionError;
use crate::identifier::{identifier_from_js_value, IdentifierWrapper};
use crate::lodash::lodash_set;
use crate::utils::WithJsError;
use crate::utils::{with_serde_to_json_value, ToSerdeJSONExt};
use crate::{with_js_error, ConversionOptions};
use crate::{DataContractWasm, MetadataWasm};

#[wasm_bindgen(js_name=ExtendedDocument)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtendedDocumentWasm(pub(crate) ExtendedDocument);

#[wasm_bindgen(js_class=ExtendedDocument)]
impl ExtendedDocumentWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(
        js_raw_document: JsValue,
        js_data_contract: &DataContractWasm,
    ) -> Result<ExtendedDocumentWasm, JsValue> {
        let mut raw_document = with_serde_to_json_value(&js_raw_document)?;

        let document_type = raw_document
            .get_string(extended_document_property_names::DOCUMENT_TYPE)
            .with_js_error()?;

        let (identifier_paths, _) = js_data_contract
            .inner()
            .get_identifiers_and_binary_paths(document_type)
            .with_js_error()?;

        // Errors are ignored. When `Buffer` crosses the WASM boundary it becomes an Array.
        // When `Identifier` crosses the WASM boundary it becomes a String. From perspective of JS
        // `Identifier` and `Buffer` are used interchangeably, so we we can expect the replacing may fail when `Buffer` is provided
        let _ = raw_document
            .replace_identifier_paths(
                identifier_paths
                    .into_iter()
                    .chain(EXTENDED_DOCUMENT_IDENTIFIER_FIELDS),
                ReplaceWith::Bytes,
            )
            .with_js_error();
        // The binary paths are not being converted, because they always should be a `Buffer`. `Buffer` is always an Array

        let document =
            ExtendedDocument::from_raw_document(raw_document, js_data_contract.to_owned().into())
                .with_js_error()?;

        Ok(document.into())
    }

    #[wasm_bindgen(js_name=getProtocolVersion)]
    pub fn get_protocol_version(&self) -> u32 {
        self.0.protocol_version
    }

    #[wasm_bindgen(js_name=getId)]
    pub fn get_id(&self) -> IdentifierWrapper {
        self.0.document.id.into()
    }

    #[wasm_bindgen(js_name=setId)]
    pub fn set_id(&mut self, js_id: IdentifierWrapper) {
        self.0.document.id = js_id.inner().buffer;
    }

    #[wasm_bindgen(js_name=getType)]
    pub fn get_type(&self) -> String {
        self.0.document_type_name.clone()
    }

    #[wasm_bindgen(js_name=getDataContractId)]
    pub fn get_data_contract_id(&self) -> IdentifierWrapper {
        self.0.data_contract_id.into()
    }

    #[wasm_bindgen(js_name=getDataContract)]
    pub fn get_data_contract(&self) -> DataContractWasm {
        self.0.data_contract.clone().into()
    }

    #[wasm_bindgen(js_name=setDataContractId)]
    pub fn set_data_contract_id(&mut self, js_data_contract_id: &JsValue) -> Result<(), JsValue> {
        let identifier = identifier_from_js_value(js_data_contract_id)?;
        self.0.data_contract_id = identifier;
        Ok(())
    }

    #[wasm_bindgen(js_name=setOwnerId)]
    pub fn set_owner_id(&mut self, owner_id: IdentifierWrapper) {
        self.0.document.owner_id = owner_id.inner().buffer;
    }

    #[wasm_bindgen(js_name=getOwnerId)]
    pub fn get_owner_id(&self) -> IdentifierWrapper {
        self.0.document.owner_id.into()
    }

    #[wasm_bindgen(js_name=setRevision)]
    pub fn set_revision(&mut self, rev: Option<u32>) {
        // TODO: js feeds Number (u32). Is casting revision to u64 safe?
        self.0.document.revision = rev.map(|r| r as Revision);
    }

    #[wasm_bindgen(js_name=getRevision)]
    pub fn get_revision(&self) -> Option<u32> {
        // TODO: js expects Number (u32). Is casting revision to u32 safe?
        self.0.document.revision.map(|r| r as u32)
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
    pub fn set_data(&mut self, d: JsValue) -> Result<(), JsValue> {
        self.0.document.properties = with_js_error!(serde_wasm_bindgen::from_value(d))?;
        Ok(())
    }

    #[wasm_bindgen(js_name=getData)]
    pub fn get_data(&mut self) -> Result<JsValue, JsValue> {
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();

        Ok(with_js_error!(self
            .0
            .document
            .properties
            .serialize(&serializer))?)
    }

    #[wasm_bindgen(js_name=set)]
    pub fn set(&mut self, path: String, js_value_to_set: JsValue) -> Result<(), JsValue> {
        let (identifier_paths, _) = self.0.get_identifiers_and_binary_paths().with_js_error()?;
        let mut value: Value = js_value_to_set.with_serde_to_json_value()?.into();
        if identifier_paths.contains(path.as_str()) {
            let identifier_value = ReplacementType::Bytes
                .replace_consume_value(value)
                .map_err(ProtocolError::ValueError)
                .with_js_error()?;
            return self.0.set(&path, identifier_value).with_js_error();
        } else {
            identifier_paths
                .into_iter()
                .try_for_each(|identifier_path| {
                    if identifier_path.starts_with(path.as_str()) {
                        let (_, suffix) = identifier_path.split_at(path.len() + 1);
                        value
                            .replace_at_path(suffix, ReplacementType::Bytes)
                            .map_err(ProtocolError::ValueError)
                            .map(|_| ())
                            .with_js_error()
                    } else {
                        Ok(())
                    }
                })?;
        }
        self.0.set(&path, value).with_js_error()
    }

    #[wasm_bindgen(js_name=get)]
    pub fn get(&mut self, path: String) -> JsValue {
        let binary_type = self.get_binary_type_of_path(&path);

        if let Some(value) = self.0.get(&path) {
            match binary_type {
                BinaryType::Identifier => {
                    if let Ok(bytes) = value.to_identifier_bytes() {
                        let id: IdentifierWrapper = Identifier::from_bytes(&bytes).unwrap().into();

                        return id.into();
                    }
                }
                BinaryType::Buffer => {
                    if let Ok(bytes) = value.to_identifier_bytes() {
                        return Buffer::from_bytes(&bytes).into();
                    }
                }
                BinaryType::None => {
                    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
                    let json_value: Option<JsonValue> = value.clone().try_into().ok();
                    if let Some(json_value) = json_value {
                        if let Ok(js_value) = json_value.serialize(&serializer) {
                            return js_value;
                        }
                    }
                }
            }
        }

        JsValue::undefined()
    }

    #[wasm_bindgen(js_name=setCreatedAt)]
    pub fn set_created_at(&mut self, ts: f64) {
        self.0.document.created_at = Some(ts as u64);
    }

    #[wasm_bindgen(js_name=setUpdatedAt)]
    pub fn set_updated_at(&mut self, ts: f64) {
        self.0.document.updated_at = Some(ts as u64);
    }

    #[wasm_bindgen(js_name=getCreatedAt)]
    pub fn get_created_at(&self) -> Option<f64> {
        self.0.document.created_at.map(|v| v as f64)
    }

    #[wasm_bindgen(js_name=getUpdatedAt)]
    pub fn get_updated_at(&self) -> Option<f64> {
        self.0.document.updated_at.map(|v| v as f64)
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

        for path in identifiers_paths
            .into_iter()
            .chain(EXTENDED_DOCUMENT_IDENTIFIER_FIELDS)
        {
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
    pub fn deep_clone(&self) -> Self {
        self.clone()
    }
}

impl ExtendedDocumentWasm {
    fn get_binary_type_of_path(&self, path: &String) -> BinaryType {
        let maybe_binary_properties = self
            .0
            .data_contract
            .get_binary_properties(&self.0.document_type_name);

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

impl From<ExtendedDocument> for ExtendedDocumentWasm {
    fn from(d: ExtendedDocument) -> Self {
        ExtendedDocumentWasm(d)
    }
}

impl From<ExtendedDocumentWasm> for ExtendedDocument {
    fn from(d: ExtendedDocumentWasm) -> Self {
        d.0
    }
}

impl From<&ExtendedDocumentWasm> for ExtendedDocument {
    fn from(d: &ExtendedDocumentWasm) -> Self {
        d.0.clone()
    }
}
