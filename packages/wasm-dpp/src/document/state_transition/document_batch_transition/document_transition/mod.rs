mod document_create_transition;
// mod document_delete_transition;
// mod document_replace_transition;

// pub use document_create_transition::*;
// pub use document_delete_transition::*;
// pub use document_replace_transition::*;

use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::platform_value::Value;
use dpp::prelude::TimestampMillis;
use dpp::state_transition::documents_batch_transition::document_transition::action_type::TransitionActionTypeGetter;
use dpp::state_transition::documents_batch_transition::document_transition::DocumentTransitionV0Methods;
use dpp::{
    prelude::Identifier,
    state_transition::documents_batch_transition::{
        document_transition::DocumentTransition, DocumentCreateTransition,
        DocumentDeleteTransition, DocumentReplaceTransition,
    },
    util::{json_schema::JsonSchemaExt, json_value::JsonValueExt},
    ProtocolError,
};
use serde::Serialize;
use serde_json::Value as JsonValue;
use wasm_bindgen::prelude::*;

use crate::{
    buffer::Buffer,
    identifier::{identifier_from_js_value, IdentifierWrapper},
    lodash::lodash_set,
    utils::{Inner, ToSerdeJSONExt, WithJsError},
    with_js_error, BinaryType, ConversionOptions, DataContractWasm,
};

#[wasm_bindgen(js_name=DocumentTransition)]
#[derive(Debug, Clone)]
pub struct DocumentTransitionWasm(DocumentTransition);

#[wasm_bindgen(js_class=DocumentTransition)]
impl DocumentTransitionWasm {
    #[wasm_bindgen(js_name=getId)]
    pub fn get_id(&self) -> IdentifierWrapper {
        self.0.get_id().to_owned().into()
    }

    #[wasm_bindgen(js_name=getType)]
    pub fn get_type(&self) -> String {
        self.0.document_type_name().to_owned()
    }

    #[wasm_bindgen(js_name=getAction)]
    pub fn get_action(&self) -> u8 {
        self.0.action_type() as u8
    }

    // #[wasm_bindgen(js_name=getDataContract)]
    // pub fn get_data_contract(&self) -> DataContractWasm {
    //     self.0.data_contract().to_owned().into()
    // }

    #[wasm_bindgen(js_name=getDataContractId)]
    pub fn get_data_contract_id(&self) -> IdentifierWrapper {
        self.0.data_contract_id().to_owned().into()
    }

    #[wasm_bindgen(js_name=setDataContractId)]
    pub fn set_data_contract_id(&mut self, js_data_contract_id: &JsValue) -> Result<(), JsValue> {
        let identifier = identifier_from_js_value(js_data_contract_id)?;
        self.0.set_data_contract_id(identifier);
        Ok(())
    }

    #[wasm_bindgen(js_name=getRevision)]
    pub fn get_revision(&self) -> JsValue {
        if let Some(revision) = self.0.revision() {
            (revision as f64).into()
        } else {
            JsValue::NULL
        }
    }

    #[wasm_bindgen(js_name=setRevision)]
    pub fn set_revision(&mut self, revision: u32) {
        self.0.set_revision(revision as u64);
    }

    #[wasm_bindgen(js_name=getCreatedAt)]
    pub fn get_created_at(&self) -> JsValue {
        if let Some(created_at) = self.0.created_at() {
            js_sys::Date::new(&JsValue::from_f64(created_at as f64)).into()
        } else {
            JsValue::NULL
        }
    }

    #[wasm_bindgen(js_name=getUpdatedAt)]
    pub fn get_updated_at(&self) -> JsValue {
        if let Some(updated_at) = self.0.updated_at() {
            js_sys::Date::new(&JsValue::from_f64(updated_at as f64)).into()
        } else {
            JsValue::NULL
        }
    }

    #[wasm_bindgen(js_name=setUpdatedAt)]
    pub fn set_updated_at(&mut self, updated_at: Option<js_sys::Date>) -> Result<(), JsValue> {
        self.0
            .set_updated_at(updated_at.map(|timestamp| timestamp.get_time() as TimestampMillis));

        Ok(())
    }

    #[wasm_bindgen(js_name=setCreatedAt)]
    pub fn set_created_at(&mut self, created_at: Option<js_sys::Date>) {
        self.0
            .set_created_at(created_at.map(|timestamp| timestamp.get_time() as TimestampMillis));
    }

    // #[wasm_bindgen(js_name=getData)]
    // pub fn get_data(&self) -> Result<JsValue, JsValue> {
    //     if let Some(data) = self.0.data() {
    //         let (identifier_paths, binary_paths) = self
    //             .0
    //             .data_contract()
    //             .get_identifiers_and_binary_paths(self.0.document_type_name())
    //             .with_js_error()?;
    //
    //         let js_value = to_object(
    //             data.clone().into(),
    //             &JsValue::NULL,
    //             identifier_paths,
    //             binary_paths,
    //         )?;
    //         Ok(js_value)
    //     } else {
    //         Ok(JsValue::NULL)
    //     }
    // }
    //
    // #[wasm_bindgen(js_name=get)]
    // pub fn get(&self, path: &str, data_contract: &DataContractWasm) -> JsValue {
    //     let binary_type = self.get_binary_type_of_path(path, data_contract);
    //
    //     if let Some(value) = self.0.get_dynamic_property(path) {
    //         match binary_type {
    //             BinaryType::Identifier => {
    //                 if let Ok(bytes) = value.to_identifier_bytes() {
    //                     let id: IdentifierWrapper = Identifier::from_bytes(&bytes).unwrap().into();
    //                     return id.into();
    //                 }
    //             }
    //             BinaryType::Buffer => {
    //                 if let Ok(bytes) = value.to_binary_bytes() {
    //                     return Buffer::from_bytes(&bytes).into();
    //                 }
    //             }
    //             BinaryType::None => {
    //                 let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    //                 if let Ok(js_value) = value.serialize(&serializer) {
    //                     return js_value;
    //                 }
    //             }
    //         }
    //     }
    //
    //     JsValue::undefined()
    // }

    // #[wasm_bindgen(js_name=toObject)]
    // pub fn to_object(
    //     &self,
    //     options: &JsValue,
    //     data_contract: &DataContractWasm,
    // ) -> Result<JsValue, JsValue> {
    //     match self.0 {
    //         DocumentTransition::Create(ref t) => DocumentCreateTransitionWasm::from(t.to_owned())
    //             .to_object(options, data_contract.inner()),
    //         DocumentTransition::Replace(ref t) => DocumentReplaceTransitionWasm::from(t.to_owned())
    //             .to_object(options, data_contract.inner()),
    //         DocumentTransition::Delete(ref t) => DocumentDeleteTransitionWasm::from(t.to_owned())
    //             .to_object(options, data_contract.inner()),
    //     }
    // }

    // #[wasm_bindgen(js_name=toJSON)]
    // pub fn to_json(&self) -> Result<JsValue, JsValue> {
    //     let json_value = self.0.to_json().with_js_error()?;
    //     with_js_error!(json_value.serialize(&serde_wasm_bindgen::Serializer::json_compatible()))
    // }
    //
    // #[wasm_bindgen(js_name=fromTransitionCreate)]
    // pub fn from_transition_create(
    //     js_create_transition: DocumentCreateTransitionWasm,
    // ) -> DocumentTransitionWasm {
    //     let transition_create: DocumentCreateTransition = js_create_transition.into();
    //     let document_transition = DocumentTransition::Create(transition_create);
    //
    //     document_transition.into()
    // }
    //
    // #[wasm_bindgen(js_name=fromTransitionReplace)]
    // pub fn from_transition_replace(
    //     js_replace_transition: DocumentReplaceTransitionWasm,
    // ) -> DocumentTransitionWasm {
    //     let transition_replace: DocumentReplaceTransition = js_replace_transition.into();
    //     let document_transition = DocumentTransition::Replace(transition_replace);
    //
    //     document_transition.into()
    // }
    //
    // #[wasm_bindgen(js_name=fromTransitionDelete)]
    // pub fn from_transition_delete(
    //     js_delete_transition: DocumentDeleteTransitionWasm,
    // ) -> DocumentTransitionWasm {
    //     let transition_delete: DocumentDeleteTransition = js_delete_transition.into();
    //     let document_transition = DocumentTransition::Delete(transition_delete);
    //
    //     document_transition.into()
    // }
}
//
// impl DocumentTransitionWasm {
//     fn get_binary_type_of_path(
//         &self,
//         path: impl AsRef<str>,
//         data_contract: DataContractWasm,
//     ) -> Result<BinaryType, JsValue> {
//         let document_type = data_contract
//             .inner()
//             .document_type_for_name(self.0.document_type_name().as_str())
//             .with_js_error()?;
//
//         if document_type.binary_paths().contains(&path) {
//             Ok(BinaryType::Buffer)
//         } else if document_type.identifier_paths().contains(&path) {
//             Ok(BinaryType::Identifier)
//         } else {
//             Ok(BinaryType::None)
//         }
//     }
// }

impl From<DocumentTransition> for DocumentTransitionWasm {
    fn from(v: DocumentTransition) -> Self {
        DocumentTransitionWasm(v)
    }
}

impl From<DocumentTransitionWasm> for DocumentTransition {
    fn from(v: DocumentTransitionWasm) -> Self {
        v.0
    }
}

impl Inner for DocumentTransitionWasm {
    type InnerItem = DocumentTransition;

    fn into_inner(self) -> Self::InnerItem {
        self.0
    }

    fn inner(&self) -> &Self::InnerItem {
        &self.0
    }

    fn inner_mut(&mut self) -> &mut Self::InnerItem {
        &mut self.0
    }
}

// pub fn from_document_transition_to_js_value(document_transition: DocumentTransition) -> JsValue {
//     match document_transition {
//         DocumentTransition::Create(create_transition) => {
//             DocumentCreateTransitionWasm::from(create_transition).into()
//         }
//         DocumentTransition::Replace(replace_transition) => {
//             DocumentReplaceTransitionWasm::from(replace_transition).into()
//         }
//         DocumentTransition::Delete(delete_transition) => {
//             DocumentDeleteTransitionWasm::from(delete_transition).into()
//         }
//     }
// }

pub(crate) fn to_object<'a>(
    value: Value,
    options: &JsValue,
    identifiers_paths: impl IntoIterator<Item = &'a str>,
    binary_paths: impl IntoIterator<Item = &'a str>,
) -> Result<JsValue, JsValue> {
    let mut value: JsonValue = value
        .try_into_validating_json()
        .map_err(ProtocolError::ValueError)
        .with_js_error()?;
    let options: ConversionOptions = if options.is_object() {
        let raw_options = options.with_serde_to_json_value()?;
        serde_json::from_value(raw_options).with_js_error()?
    } else {
        Default::default()
    };

    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    let js_value = value.serialize(&serializer)?;

    for path in identifiers_paths.into_iter() {
        if let Ok(bytes) = value.remove_value_at_path_into::<Vec<u8>>(path.clone()) {
            let buffer = Buffer::from_bytes_owned(bytes);
            if !options.skip_identifiers_conversion {
                lodash_set(&js_value, path, buffer.into());
            } else {
                let id = IdentifierWrapper::new(buffer.into());
                lodash_set(&js_value, path, id.into());
            }
        }
    }

    for path in binary_paths.into_iter() {
        if let Ok(bytes) = value.remove_value_at_path_into::<Vec<u8>>(path) {
            let buffer = Buffer::from_bytes(&bytes);
            lodash_set(&js_value, path, buffer.into());
        }
    }

    Ok(js_value)
}
