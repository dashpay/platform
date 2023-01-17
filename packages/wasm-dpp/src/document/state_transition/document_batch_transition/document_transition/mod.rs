mod document_create_transition;
mod document_delete_transition;
mod document_replace_transition;

pub use document_create_transition::*;
pub use document_delete_transition::*;
pub use document_replace_transition::*;

use dpp::{
    document::document_transition::{DocumentTransitionExt, DocumentTransitionObjectLike},
    prelude::{DocumentTransition, Identifier},
    util::{json_schema::JsonSchemaExt, json_value::JsonValueExt},
};
use serde::Serialize;
use serde_json::Value;
use wasm_bindgen::prelude::*;

use crate::{
    buffer::Buffer,
    conversion::ConversionOptions,
    identifier::IdentifierWrapper,
    lodash::lodash_set,
    utils::{ToSerdeJSONExt, WithJsError},
    with_js_error, BinaryType, DataContractWasm,
};

#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct DocumentTransitionWasm(DocumentTransition);

#[wasm_bindgen(js_class=DocumentTransitionWasm)]
impl DocumentTransitionWasm {
    #[wasm_bindgen(js_name=getId)]
    pub fn get_id(&self) -> IdentifierWrapper {
        self.0.get_id().to_owned().into()
    }

    #[wasm_bindgen(js_name=getType)]
    pub fn get_type(&self) -> String {
        self.0.get_document_type().to_owned()
    }

    #[wasm_bindgen(js_name=getAction)]
    pub fn get_action(&self) -> u8 {
        self.0.get_action().into()
    }

    #[wasm_bindgen(js_name=getDataContract)]
    pub fn get_data_contract(&self) -> DataContractWasm {
        self.0.get_data_contract().to_owned().into()
    }

    #[wasm_bindgen(js_name=getDataContractId)]
    pub fn get_data_contract_id(&self) -> IdentifierWrapper {
        self.0.get_data_contract_id().to_owned().into()
    }

    #[wasm_bindgen(js_name=getRevision)]
    pub fn get_revision(&self) -> JsValue {
        if let Some(revision) = self.0.get_revision() {
            revision.into()
        } else {
            JsValue::NULL
        }
    }

    #[wasm_bindgen(js_name=getUpdatedAt)]
    pub fn get_updated_at(&self) -> JsValue {
        if let Some(revision) = self.0.get_updated_at() {
            revision.into()
        } else {
            JsValue::NULL
        }
    }

    #[wasm_bindgen(js_name=getData)]
    pub fn get_data(&self) -> Result<JsValue, JsValue> {
        if let Some(data) = self.0.get_data() {
            let (identifier_paths, binary_paths) = self
                .0
                .get_data_contract()
                .get_identifiers_and_binary_paths(self.0.get_document_type());
            let js_value = convert_to_object(
                data.to_owned(),
                &JsValue::NULL,
                identifier_paths,
                binary_paths,
            )?;
            Ok(js_value)
        } else {
            Ok(JsValue::NULL)
        }
    }

    #[wasm_bindgen(js_name=get)]
    pub fn get(&self, path: &str) -> JsValue {
        let binary_type = self.get_binary_type_of_path(path);

        if let Some(value) = self.0.get_dynamic_property(path) {
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

    #[wasm_bindgen(js_name=getObject)]
    pub fn to_object(&self, options: &JsValue) -> Result<JsValue, JsValue> {
        match self.0 {
            DocumentTransition::Create(ref t) => {
                DocumentCreateTransitionWasm::from(t.to_owned()).to_object(options)
            }
            DocumentTransition::Replace(ref t) => {
                DocumentReplaceTransitionWasm::from(t.to_owned()).to_object(options)
            }
            DocumentTransition::Delete(ref t) => {
                DocumentDeleteTransitionWasm::from(t.to_owned()).to_object(options)
            }
        }
    }

    #[wasm_bindgen(js_name=toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        let json_value = self.0.to_json().with_js_error()?;
        with_js_error!(json_value.serialize(&serde_wasm_bindgen::Serializer::json_compatible()))
    }
}

impl DocumentTransitionWasm {
    fn get_binary_type_of_path(&self, path: impl AsRef<str>) -> BinaryType {
        let maybe_binary_properties = self
            .0
            .get_data_contract()
            .get_binary_properties(self.0.get_document_type());

        if let Ok(binary_properties) = maybe_binary_properties {
            if let Some(data) = binary_properties.get(path.as_ref()) {
                if data.is_type_of_identifier() {
                    return BinaryType::Identifier;
                }
                return BinaryType::Buffer;
            }
        }
        BinaryType::None
    }
}

impl From<DocumentTransition> for DocumentTransitionWasm {
    fn from(v: DocumentTransition) -> Self {
        DocumentTransitionWasm(v)
    }
}

pub fn from_document_transition_to_js_value(document_transition: DocumentTransition) -> JsValue {
    match document_transition {
        DocumentTransition::Create(create_transition) => {
            DocumentCreateTransitionWasm::from(create_transition).into()
        }
        DocumentTransition::Replace(replace_transition) => {
            DocumentReplaceTransitionWasm::from(replace_transition).into()
        }
        DocumentTransition::Delete(delete_transition) => {
            DocumentDeleteTransitionWasm::from(delete_transition).into()
        }
    }
}

pub(crate) fn convert_to_object<'a>(
    value: Value,
    options: &JsValue,
    identifiers_paths: impl IntoIterator<Item = &'a str>,
    binary_paths: impl IntoIterator<Item = &'a str>,
) -> Result<JsValue, JsValue> {
    let mut value = value;
    let options: ConversionOptions = if options.is_object() {
        let raw_options = options.with_serde_to_json_value()?;
        serde_json::from_value(raw_options).with_js_error()?
    } else {
        Default::default()
    };

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

    for path in binary_paths.into_iter() {
        if let Ok(bytes) = value.remove_path_into::<Vec<u8>>(path) {
            let buffer = Buffer::from_bytes(&bytes);
            lodash_set(&js_value, path, buffer.into());
        }
    }

    Ok(js_value)
}
