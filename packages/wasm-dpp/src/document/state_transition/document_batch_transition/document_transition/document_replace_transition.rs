use std::convert;
use std::convert::TryInto;

use dpp::document::document_transition::document_base_transition::JsonValue;

use dpp::platform_value::btreemap_extensions::{
    BTreeValueMapHelper, BTreeValueMapPathHelper, BTreeValueMapReplacementPathHelper,
};
use dpp::platform_value::ReplacementType;
use dpp::prelude::Revision;
use dpp::{
    document::document_transition::{
        document_replace_transition, DocumentReplaceTransition, DocumentTransitionObjectLike,
    },
    prelude::{DataContract, Identifier},
    util::json_schema::JsonSchemaExt,
    ProtocolError,
};
use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::{
    buffer::Buffer,
    document_batch_transition::document_transition::to_object,
    identifier::IdentifierWrapper,
    lodash::lodash_set,
    utils::{ToSerdeJSONExt, WithJsError},
    BinaryType, DataContractWasm,
};

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
        let data_contract: DataContract = data_contract.clone().into();
        let mut value = raw_object.with_serde_to_platform_value_map()?;
        let document_type = value
            .get_string(dpp::document::extended_document::property_names::DOCUMENT_TYPE)
            .map_err(ProtocolError::ValueError)
            .with_js_error()?;

        let (identifier_paths, _): (Vec<_>, Vec<_>) = data_contract
            .get_identifiers_and_binary_paths_owned(document_type.as_str())
            .with_js_error()?;
        value
            .replace_at_paths(identifier_paths, ReplacementType::Identifier)
            .map_err(ProtocolError::ValueError)
            .with_js_error()?;
        let transition =
            DocumentReplaceTransition::from_value_map(value, data_contract).with_js_error()?;

        Ok(transition.into())
    }

    #[wasm_bindgen(js_name=getAction)]
    pub fn action(&self) -> u8 {
        self.inner.base.action as u8
    }

    #[wasm_bindgen(js_name=getRevision)]
    pub fn revision(&self) -> Revision {
        self.inner.revision
    }

    #[wasm_bindgen(js_name=getUpdatedAt)]
    pub fn updated_at(&self) -> Option<js_sys::Date> {
        self.inner
            .updated_at
            .map(|timestamp| js_sys::Date::new(&JsValue::from_f64(timestamp as f64)))
    }

    #[wasm_bindgen(js_name=toObject)]
    pub fn to_object(&self, options: &JsValue) -> Result<JsValue, JsValue> {
        let (identifiers_paths, binary_paths) = self
            .inner
            .base
            .data_contract
            .get_identifiers_and_binary_paths(&self.inner.base.document_type_name)
            .with_js_error()?;

        to_object(
            self.inner.to_object().with_js_error()?,
            options,
            identifiers_paths
                .into_iter()
                .chain(document_replace_transition::IDENTIFIER_FIELDS),
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
    pub fn get_data(&self) -> Result<JsValue, JsValue> {
        let data = if let Some(ref data) = self.inner.data {
            data
        } else {
            return Ok(JsValue::undefined());
        };

        let js_value = data.serialize(&serde_wasm_bindgen::Serializer::json_compatible())?;
        let (identifier_paths, binary_paths) = self
            .inner
            .base
            .data_contract
            .get_identifiers_and_binary_paths(&self.inner.base.document_type_name)
            .with_js_error()?;

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
        self.inner.base.id.into()
    }

    #[wasm_bindgen(js_name=getType)]
    pub fn document_type(&self) -> String {
        self.inner.base.document_type_name.clone()
    }

    #[wasm_bindgen(js_name=getDataContract)]
    pub fn data_contract(&self) -> DataContractWasm {
        self.inner.base.data_contract.clone().into()
    }

    #[wasm_bindgen(js_name=getDataContractId)]
    pub fn data_contract_id(&self) -> IdentifierWrapper {
        self.inner.base.data_contract.id.into()
    }

    #[wasm_bindgen(js_name=get)]
    pub fn get(&self, path: String) -> Result<JsValue, JsValue> {
        let document_data = if let Some(ref data) = self.inner.data {
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
