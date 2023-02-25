use dpp::dashcore::anyhow::Context;
use dpp::prelude::{Identifier, Revision};
use dpp::util::json_schema::JsonSchemaExt;
use dpp::util::json_value::{JsonValueExt, ReplaceWith};
use dpp::util::string_encoding::Encoding;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use wasm_bindgen::prelude::*;

use crate::buffer::Buffer;
use crate::errors::RustConversionError;
use crate::identifier::IdentifierWrapper;
use crate::lodash::lodash_set;
use crate::utils::WithJsError;
use crate::utils::{with_serde_to_json_value, with_serde_to_platform_value, ToSerdeJSONExt};
use crate::with_js_error;
use crate::{DataContractWasm, MetadataWasm};

pub mod errors;
pub use state_transition::*;
mod document_in_state_transition;
mod factory;
pub mod state_transition;
mod validator;

pub use document_batch_transition::{DocumentsBatchTransitionWASM, DocumentsContainer};
pub use document_in_state_transition::DocumentInStateTransitionWasm;
use dpp::data_contract::{DataContract, DriveContractExt};
use dpp::document::{
    document_in_state_transition_property_names, Document,
    DOCUMENT_IN_STATE_TRANSITION_IDENTIFIER_FIELDS,
};
use dpp::identity::TimestampMillis;
use dpp::platform_value::Value;
use dpp::ProtocolError;
pub use factory::DocumentFactoryWASM;
use serde_json::Value as JsonValue;
pub use validator::DocumentValidatorWasm;

pub(super) enum BinaryType {
    Identifier,
    Buffer,
    None,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConversionOptions {
    skip_identifiers_conversion: bool,
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
        let mut raw_document = with_serde_to_json_value(&js_raw_document)?;

        let document_type = raw_document
            .get_string(document_in_state_transition_property_names::DOCUMENT_TYPE)
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
                    .chain(DOCUMENT_IN_STATE_TRANSITION_IDENTIFIER_FIELDS),
                ReplaceWith::Bytes,
            )
            .with_js_error();
        // The binary paths are not being converted, because they always should be a `Buffer`. `Buffer` is always an Array

        let document = Document::from_raw_json_document(raw_document).with_js_error()?;

        Ok(document.into())
    }

    #[wasm_bindgen(js_name=getId)]
    pub fn get_id(&self) -> IdentifierWrapper {
        self.0.id.into()
    }

    #[wasm_bindgen(js_name=setId)]
    pub fn set_id(&mut self, js_id: IdentifierWrapper) {
        self.0.id = js_id.inner().to_buffer();
    }

    #[wasm_bindgen(js_name=setOwnerId)]
    pub fn set_owner_id(&mut self, owner_id: IdentifierWrapper) {
        self.0.owner_id = owner_id.inner().to_buffer();
    }

    #[wasm_bindgen(js_name=getOwnerId)]
    pub fn get_owner_id(&self) -> IdentifierWrapper {
        self.0.owner_id.into()
    }

    #[wasm_bindgen(js_name=setRevision)]
    pub fn set_revision(&mut self, revision: Option<Revision>) {
        self.0.revision = revision
    }

    #[wasm_bindgen(js_name=getRevision)]
    pub fn get_revision(&self) -> Option<Revision> {
        self.0.revision
    }

    #[wasm_bindgen(js_name=setData)]
    pub fn set_properties(&mut self, d: JsValue) -> Result<(), JsValue> {
        self.0.properties = with_js_error!(serde_wasm_bindgen::from_value(d))?;
        Ok(())
    }

    #[wasm_bindgen(js_name=getData)]
    pub fn get_properties(&mut self) -> Result<JsValue, JsValue> {
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();

        Ok(with_js_error!(self.0.properties.serialize(&serializer))?)
    }

    #[wasm_bindgen(js_name=set)]
    pub fn set(&mut self, path: String, js_value_to_set: JsValue) -> Result<(), JsValue> {
        let value = js_value_to_set.with_serde_to_platform_value()?;
        Ok(self.0.set(&path, value))
    }

    #[wasm_bindgen(js_name=get)]
    pub fn get(
        &mut self,
        path: String,
        data_contract: DataContractWasm,
        document_type_name: String,
    ) -> Result<JsValue, JsValue> {
        let binary_type = self.get_binary_type_of_path(&path, data_contract, document_type_name);

        if let Some(value) = self.0.get(&path) {
            let json_value_result: Result<JsonValue, ProtocolError> =
                value.clone().try_into().map_err(ProtocolError::ValueError);
            let json_value = json_value_result.with_js_error()?;
            match binary_type {
                BinaryType::Identifier => {
                    if let Ok(bytes) = serde_json::from_value::<Vec<u8>>(json_value) {
                        let id: IdentifierWrapper = Identifier::from_bytes(&bytes).unwrap().into();

                        return Ok(id.into());
                    }
                }
                BinaryType::Buffer => {
                    if let Ok(bytes) = serde_json::from_value::<Vec<u8>>(json_value) {
                        return Ok(Buffer::from_bytes(&bytes).into());
                    }
                }
                BinaryType::None => {
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
    pub fn set_created_at(&mut self, ts: f64) {
        self.0.created_at = Some(ts as TimestampMillis);
    }

    #[wasm_bindgen(js_name=setUpdatedAt)]
    pub fn set_updated_at(&mut self, ts: f64) {
        self.0.updated_at = Some(ts as TimestampMillis);
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
            let raw_options = options.with_serde_to_json_value()?;
            serde_json::from_value(raw_options).with_js_error()?
        } else {
            Default::default()
        };
        let mut value = self
            .0
            .to_object(&data_contract.0, document_type_name)
            .with_js_error()?;

        let (identifiers_paths, binary_paths) = self
            .0
            .get_identifiers_and_binary_paths(&data_contract.0, document_type_name)
            .with_js_error()?;
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        let js_value = value.serialize(&serializer)?;

        for path in identifiers_paths.into_iter() {
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
    pub fn deep_clone(&self) -> Self {
        self.clone()
    }
}

impl DocumentWasm {
    fn get_binary_type_of_path(
        &self,
        path: &String,
        data_contract: DataContractWasm,
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

impl From<Document> for DocumentWasm {
    fn from(d: Document) -> Self {
        DocumentWasm(d)
    }
}
