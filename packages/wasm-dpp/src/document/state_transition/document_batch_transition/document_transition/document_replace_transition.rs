use std::convert;
use std::convert::TryInto;

use serde_json::Value as JsonValue;

use dpp::platform_value::btreemap_extensions::{
    BTreeValueMapHelper, BTreeValueMapPathHelper, BTreeValueMapReplacementPathHelper,
};
use dpp::platform_value::ReplacementType;
use dpp::prelude::Revision;
use dpp::{
    prelude::{DataContract, Identifier},
    state_transition::documents_batch_transition::{
        document_replace_transition, DocumentReplaceTransition,
    },
    util::json_schema::JsonSchemaExt,
    ProtocolError,
};
use serde::Serialize;
use wasm_bindgen::prelude::*;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::state_transition::documents_batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use dpp::state_transition::documents_batch_transition::document_replace_transition::v0::v0_methods::DocumentReplaceTransitionV0Methods;

use crate::{
    buffer::Buffer,
    document::state_transition::document_batch_transition::document_transition::to_object,
    identifier::IdentifierWrapper,
    lodash::lodash_set,
    utils::{ToSerdeJSONExt, WithJsError},
    BinaryType, DataContractWasm,
};
use dpp::state_transition::documents_batch_transition::document_transition::action_type::DocumentTransitionActionType;

#[wasm_bindgen(js_name=DocumentReplaceTransition)]
#[derive(Debug, Clone)]
pub struct DocumentReplaceTransitionWasm {
    inner: DocumentReplaceTransition,
}

impl From<DocumentReplaceTransition> for DocumentReplaceTransitionWasm {
    fn from(v: DocumentReplaceTransition) -> Self {
        Self { inner: v }
    }
}

impl From<DocumentReplaceTransitionWasm> for DocumentReplaceTransition {
    fn from(v: DocumentReplaceTransitionWasm) -> Self {
        v.inner
    }
}

#[wasm_bindgen(js_class=DocumentReplaceTransition)]
impl DocumentReplaceTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn from_object(
        raw_object: JsValue,
        data_contract: &DataContractWasm,
    ) -> Result<DocumentReplaceTransitionWasm, JsValue> {
        let mut value = raw_object.with_serde_to_platform_value_map()?;
        let document_type_name = value
            .get_string(dpp::document::extended_document::property_names::DOCUMENT_TYPE_NAME)
            .map_err(ProtocolError::ValueError)
            .with_js_error()?;

        let document_type = data_contract
            .inner()
            .document_type_for_name(document_type_name)
            .with_js_error()?;
        let identifier_paths = document_type.identifier_paths();
        let binary_paths = document_type.binary_paths();

        value
            .replace_at_paths(identifier_paths, ReplacementType::Identifier)
            .map_err(ProtocolError::ValueError)
            .with_js_error()?;

        value
            .replace_at_paths(binary_paths, ReplacementType::BinaryBytes)
            .map_err(ProtocolError::ValueError)
            .with_js_error()?;
        let transition =
            DocumentReplaceTransition::from_value_map(value, data_contract).with_js_error()?;

        Ok(transition.into())
    }

    #[wasm_bindgen(js_name=getAction)]
    pub fn action(&self) -> u8 {
        DocumentTransitionActionType::Replace as u8
    }

    #[wasm_bindgen(js_name=getRevision)]
    pub fn revision(&self) -> Revision {
        self.inner.revision()
    }

    #[wasm_bindgen(js_name=getUpdatedAt)]
    pub fn updated_at(&self) -> Option<js_sys::Date> {
        self.inner
            .updated_at()
            .map(|timestamp| js_sys::Date::new(&JsValue::from_f64(timestamp as f64)))
    }

    #[wasm_bindgen(js_name=toObject)]
    pub fn to_object(
        &self,
        options: &JsValue,
        data_contract: DataContractWasm,
    ) -> Result<JsValue, JsValue> {
        let document_type = data_contract
            .inner()
            .document_type_for_name(self.inner.base().document_type_name())
            .with_js_error()?;
        let identifier_paths = document_type.identifier_paths();
        let binary_paths = document_type.binary_paths();

        to_object(
            self.inner.to_object().with_js_error()?,
            options,
            identifier_paths
                .into_iter()
                .chain(document_replace_transition::v0::IDENTIFIER_FIELDS),
            binary_paths,
        )
    }

    #[wasm_bindgen(js_name=toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        let value = self.inner.to_json().with_js_error()?;
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        let js_value = value.serialize(&serializer)?;
        Ok(js_value)
    }

    // AbstractDataDocumentTransition
    #[wasm_bindgen(js_name=getData)]
    pub fn get_data(&self, data_contract: DataContractWasm) -> Result<JsValue, JsValue> {
        let data = if let Some(ref data) = self.inner.data() {
            data
        } else {
            return Ok(JsValue::undefined());
        };

        let js_value = data.serialize(&serde_wasm_bindgen::Serializer::json_compatible())?;
        let document_type = data_contract
            .inner()
            .document_type_for_name(self.inner.base().document_type_name())
            .with_js_error()?;
        let identifier_paths = document_type.identifier_paths();
        let binary_paths = document_type.binary_paths();

        for path in identifier_paths {
            let bytes = data
                .get_identifier_bytes_at_path(path)
                .map_err(ProtocolError::ValueError)
                .with_js_error()?;
            let id = <IdentifierWrapper as convert::From<Identifier>>::from(
                Identifier::from_bytes(&bytes).unwrap(),
            );
            lodash_set(&js_value, path, id.into());
        }
        for path in binary_paths {
            let bytes = data
                .get_binary_bytes_at_path(path)
                .map_err(ProtocolError::ValueError)
                .with_js_error()?;
            let buffer = Buffer::from_bytes(&bytes);
            lodash_set(&js_value, path, buffer.into());
        }

        Ok(js_value)
    }

    // AbstractDocumentTransition
    #[wasm_bindgen(js_name=getId)]
    pub fn id(&self) -> IdentifierWrapper {
        self.inner.base().id().into()
    }

    #[wasm_bindgen(js_name=getType)]
    pub fn document_type(&self) -> String {
        self.inner.base().document_type_name().clone()
    }

    #[wasm_bindgen(js_name=getDataContractId)]
    pub fn data_contract_id(&self) -> IdentifierWrapper {
        self.inner.base().data_contract_id().into()
    }

    #[wasm_bindgen(js_name=get)]
    pub fn get(&self, path: String) -> Result<JsValue, JsValue> {
        let document_data = if let Some(ref data) = self.inner.data() {
            data
        } else {
            return Ok(JsValue::undefined());
        };

        let value = if let Ok(value) = document_data.get_at_path(&path) {
            value.to_owned()
        } else {
            return Ok(JsValue::undefined());
        };

        match self.get_binary_type_of_path(&path) {
            BinaryType::Buffer => {
                let bytes: Vec<u8> = serde_json::from_value(
                    value
                        .try_into()
                        .map_err(ProtocolError::ValueError)
                        .with_js_error()?,
                )
                .unwrap();
                let buffer = Buffer::from_bytes(&bytes);
                return Ok(buffer.into());
            }
            BinaryType::Identifier => {
                let bytes: Vec<u8> = serde_json::from_value(
                    value
                        .try_into()
                        .map_err(ProtocolError::ValueError)
                        .with_js_error()?,
                )
                .unwrap();
                let id = <IdentifierWrapper as convert::From<Identifier>>::from(
                    Identifier::from_bytes(&bytes).unwrap(),
                );
                return Ok(id.into());
            }
            BinaryType::None => {
                // Do nothing. If is 'None' it means that binary may contain binary data
                // or may not captain it at all
            }
        }

        let json_value: JsonValue = value
            .clone()
            .try_into()
            .map_err(ProtocolError::ValueError)
            .with_js_error()?;
        let map = value
            .to_btree_ref_string_map()
            .map_err(ProtocolError::ValueError)
            .with_js_error()?;
        let js_value = json_value.serialize(&serde_wasm_bindgen::Serializer::json_compatible())?;
        let (identifier_paths, binary_paths) = self
            .inner
            .base
            .data_contract
            .get_identifiers_and_binary_paths(&self.inner.base.document_type_name)
            .with_js_error()?;

        for property_path in identifier_paths {
            if property_path.starts_with(&path) {
                let (_, suffix) = property_path.split_at(path.len() + 1);

                if let Some(bytes) = map
                    .get_optional_bytes_at_path(suffix)
                    .map_err(ProtocolError::ValueError)
                    .with_js_error()?
                {
                    let id = <IdentifierWrapper as convert::From<Identifier>>::from(
                        Identifier::from_bytes(&bytes).unwrap(),
                    );
                    lodash_set(&js_value, suffix, id.into());
                }
            }
        }

        for property_path in binary_paths {
            if property_path.starts_with(&path) {
                let (_, suffix) = property_path.split_at(path.len() + 1);

                if let Some(bytes) = map
                    .get_optional_bytes_at_path(suffix)
                    .map_err(ProtocolError::ValueError)
                    .with_js_error()?
                {
                    let buffer = Buffer::from_bytes(&bytes);
                    lodash_set(&js_value, suffix, buffer.into());
                }
            }
        }

        Ok(js_value)
    }
}

impl DocumentReplaceTransitionWasm {
    fn get_binary_type_of_path(&self, path: &String) -> BinaryType {
        let maybe_binary_properties = self
            .inner
            .base
            .data_contract
            .get_binary_properties(&self.inner.base.document_type_name);

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
