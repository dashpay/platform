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
use dpp::prelude::Revision;
use dpp::fee::Credits;
use dpp::platform_value::converter::serde_json::BTreeValueJsonConverter;
use dpp::state_transition::documents_batch_transition::document_base_transition::DocumentBaseTransition;
use dpp::state_transition::documents_batch_transition::document_replace_transition::v0::v0_methods::DocumentReplaceTransitionV0Methods;
use dpp::state_transition::documents_batch_transition::document_transition::document_purchase_transition::v0::v0_methods::DocumentPurchaseTransitionV0Methods;
use dpp::state_transition::documents_batch_transition::document_transition::document_transfer_transition::v0::v0_methods::DocumentTransferTransitionV0Methods;
use dpp::state_transition::documents_batch_transition::document_transition::document_update_price_transition::v0::v0_methods::DocumentUpdatePriceTransitionV0Methods;
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

    #[wasm_bindgen(js_name=getData)]
    pub fn get_data(&self) -> JsValue {
        match &self.0 {
            DocumentTransition::Create(document_create_transition) => {
                let json_value = document_create_transition.data().to_json_value().unwrap();
                json_value
                    .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
                    .unwrap()
            }
            DocumentTransition::Replace(document_replace_transition) => {
                let json_value = document_replace_transition.data().to_json_value().unwrap();
                json_value
                    .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
                    .unwrap()
            }
            DocumentTransition::Delete(document_delete_transition) => JsValue::null(),
            DocumentTransition::Transfer(document_transfer_transition) => JsValue::null(),
            DocumentTransition::UpdatePrice(document_update_price_transition) => JsValue::null(),
            DocumentTransition::Purchase(document_purchase_transition) => JsValue::null(),
        }
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

    #[wasm_bindgen(js_name=getIdentityContractNonce)]
    pub fn get_identity_contract_nonce(&self) -> JsValue {
        match self.0.base() {
            DocumentBaseTransition::V0(v0) => JsValue::from(v0.identity_contract_nonce),
        }
    }

    #[wasm_bindgen(js_name=getRevision)]
    pub fn get_revision(&self) -> Option<Revision> {
        self.0.revision()
    }
    #[wasm_bindgen(js_name=getEntropy)]
    pub fn get_entropy(&self) -> Option<Vec<u8>> {
        self.0.entropy()
    }

    #[wasm_bindgen(js_name=get_price)]
    pub fn get_price(&self) -> Option<Credits> {
        match &self.0 {
            DocumentTransition::Create(create) => None,
            DocumentTransition::Replace(_) => None,
            DocumentTransition::Delete(_) => None,
            DocumentTransition::Transfer(_) => None,
            DocumentTransition::UpdatePrice(update_price) => Some(update_price.price()),
            DocumentTransition::Purchase(purchase) => Some(purchase.price()),
        }
    }

    #[wasm_bindgen(js_name=getReceiverId)]
    pub fn get_receiver_id(&self) -> Option<IdentifierWrapper> {
        match &self.0 {
            DocumentTransition::Create(create) => None,
            DocumentTransition::Replace(_) => None,
            DocumentTransition::Delete(_) => None,
            DocumentTransition::Transfer(transfer) => Some(transfer.recipient_owner_id().into()),
            DocumentTransition::UpdatePrice(update_price) => None,
            DocumentTransition::Purchase(purchase) => None,
        }
    }

    #[wasm_bindgen(js_name=setRevision)]
    pub fn set_revision(&mut self, revision: u64) {
        self.0.set_revision(revision);
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

    #[wasm_bindgen(js_name=getPrefundedVotingBalance)]
    pub fn get_prefunded_voting_balance(&self) -> Result<JsValue, JsValue> {
        match &self.0 {
            DocumentTransition::Create(create_transition) => {
                let prefunded_voting_balance = create_transition.prefunded_voting_balance().clone();

                if prefunded_voting_balance.is_none() {
                    return Ok(JsValue::null());
                }

                let (index_name, credits) = prefunded_voting_balance.unwrap();

                let js_object = js_sys::Object::new();

                js_sys::Reflect::set(
                    &js_object,
                    &JsValue::from_str(&index_name),
                    &JsValue::from(credits),
                )?;

                Ok(JsValue::from(js_object))
            }
            _ => Ok(JsValue::null()),
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
