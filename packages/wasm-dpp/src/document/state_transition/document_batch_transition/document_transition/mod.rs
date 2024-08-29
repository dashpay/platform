mod document_create_transition;
// mod document_delete_transition;
// mod document_replace_transition;

// pub use document_create_transition::*;
// pub use document_delete_transition::*;
// pub use document_replace_transition::*;

use dpp::platform_value::Value;
use dpp::state_transition::documents_batch_transition::document_create_transition::v0::v0_methods::DocumentCreateTransitionV0Methods;
use dpp::state_transition::documents_batch_transition::document_transition::action_type::TransitionActionTypeGetter;
use dpp::state_transition::documents_batch_transition::document_transition::DocumentTransitionV0Methods;
use dpp::{
    state_transition::documents_batch_transition::document_transition::DocumentTransition,
    util::json_value::JsonValueExt, ProtocolError,
};
use serde::Serialize;
use serde_json::Value as JsonValue;
use wasm_bindgen::prelude::*;

use crate::{
    buffer::Buffer,
    identifier::{identifier_from_js_value, IdentifierWrapper},
    lodash::lodash_set,
    utils::{Inner, ToSerdeJSONExt, WithJsError},
    ConversionOptions,
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

    #[wasm_bindgen(js_name=hasPrefundedBalance)]
    pub fn has_prefunded_balance(&self) -> bool {
        match &self.0 {
            DocumentTransition::Create(create_transition) => {
                create_transition.prefunded_voting_balance().is_some()
            }
            _ => false,
        }
    }
}

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

#[allow(dead_code)]
#[deprecated(note = "This function is marked as unused.")]
#[allow(deprecated)]
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
        if let Ok(bytes) = value.remove_value_at_path_into::<Vec<u8>>(path) {
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
