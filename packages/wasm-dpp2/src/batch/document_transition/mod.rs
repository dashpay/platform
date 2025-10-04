use crate::batch::document_transitions::create::DocumentCreateTransitionWASM;
use crate::batch::document_transitions::delete::DocumentDeleteTransitionWASM;
use crate::batch::document_transitions::purchase::DocumentPurchaseTransitionWASM;
use crate::batch::document_transitions::replace::DocumentReplaceTransitionWASM;
use crate::batch::document_transitions::transfer::DocumentTransferTransitionWASM;
use crate::batch::document_transitions::update_price::DocumentUpdatePriceTransitionWASM;
use dpp::prelude::{Identifier, IdentityNonce, Revision};
use dpp::state_transition::batch_transition::batched_transition::document_transition::{
    DocumentTransition, DocumentTransitionV0Methods,
};
use dpp::state_transition::batch_transition::batched_transition::document_transition_action_type::{DocumentTransitionActionType, DocumentTransitionActionTypeGetter};
use crate::enums::batch::batch_enum::BatchTypeWASM;
use crate::identifier::IdentifierWASM;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone)]
#[wasm_bindgen(js_name = "DocumentTransitionWASM")]
pub struct DocumentTransitionWASM(DocumentTransition);

impl From<DocumentTransition> for DocumentTransitionWASM {
    fn from(transition: DocumentTransition) -> Self {
        DocumentTransitionWASM(transition)
    }
}

impl From<DocumentTransitionWASM> for DocumentTransition {
    fn from(transition: DocumentTransitionWASM) -> Self {
        transition.0
    }
}

#[wasm_bindgen]
impl DocumentTransitionWASM {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "DocumentTransitionWASM".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "DocumentTransitionWASM".to_string()
    }

    #[wasm_bindgen(getter = "actionType")]
    pub fn get_action_type(&self) -> String {
        BatchTypeWASM::from(self.0.action_type()).into()
    }

    #[wasm_bindgen(getter = "actionTypeNumber")]
    pub fn get_action_type_number(&self) -> u8 {
        match self.0.action_type() {
            DocumentTransitionActionType::Create => 0,
            DocumentTransitionActionType::Replace => 1,
            DocumentTransitionActionType::Delete => 2,
            DocumentTransitionActionType::Transfer => 3,
            DocumentTransitionActionType::Purchase => 4,
            DocumentTransitionActionType::UpdatePrice => 5,
            DocumentTransitionActionType::IgnoreWhileBumpingRevision => 6,
        }
    }

    #[wasm_bindgen(getter = "dataContractId")]
    pub fn get_data_contract_id(&self) -> IdentifierWASM {
        self.0.data_contract_id().into()
    }

    #[wasm_bindgen(getter = "id")]
    pub fn get_id(&self) -> IdentifierWASM {
        self.0.get_id().into()
    }

    #[wasm_bindgen(getter = "documentTypeName")]
    pub fn get_document_type_name(&self) -> String {
        self.0.document_type_name().clone()
    }

    #[wasm_bindgen(getter = "identityContractNonce")]
    pub fn get_identity_contract_nonce(&self) -> IdentityNonce {
        self.0.identity_contract_nonce()
    }

    #[wasm_bindgen(getter = "revision")]
    pub fn get_revision(&self) -> Option<Revision> {
        self.0.revision()
    }

    #[wasm_bindgen(getter = "entropy")]
    pub fn get_entropy(&self) -> Option<Vec<u8>> {
        self.0.entropy()
    }

    #[wasm_bindgen(getter = "createTransition")]
    pub fn get_create_transition(&self) -> Result<DocumentCreateTransitionWASM, JsValue> {
        match self.0.clone() {
            DocumentTransition::Create(create) => Ok(DocumentCreateTransitionWASM::from(create)),
            _ => Err(JsValue::undefined()),
        }
    }

    #[wasm_bindgen(getter = "replaceTransition")]
    pub fn get_replace_transition(&self) -> Result<DocumentReplaceTransitionWASM, JsValue> {
        match self.0.clone() {
            DocumentTransition::Replace(replace) => {
                Ok(DocumentReplaceTransitionWASM::from(replace))
            }
            _ => Err(JsValue::undefined()),
        }
    }

    #[wasm_bindgen(getter = "deleteTransition")]
    pub fn get_delete_transition(&self) -> Result<DocumentDeleteTransitionWASM, JsValue> {
        match self.0.clone() {
            DocumentTransition::Delete(delete) => Ok(DocumentDeleteTransitionWASM::from(delete)),
            _ => Err(JsValue::undefined()),
        }
    }

    #[wasm_bindgen(getter = "purchaseTransition")]
    pub fn get_purchase_transition(&self) -> Result<DocumentPurchaseTransitionWASM, JsValue> {
        match self.0.clone() {
            DocumentTransition::Purchase(purchase) => {
                Ok(DocumentPurchaseTransitionWASM::from(purchase))
            }
            _ => Err(JsValue::undefined()),
        }
    }

    #[wasm_bindgen(getter = "transferTransition")]
    pub fn get_transfer_transition(&self) -> Result<DocumentTransferTransitionWASM, JsValue> {
        match self.0.clone() {
            DocumentTransition::Transfer(transfer) => {
                Ok(DocumentTransferTransitionWASM::from(transfer))
            }
            _ => Err(JsValue::undefined()),
        }
    }

    #[wasm_bindgen(getter = "updatePriceTransition")]
    pub fn get_update_price_transition(
        &self,
    ) -> Result<DocumentUpdatePriceTransitionWASM, JsValue> {
        match self.0.clone() {
            DocumentTransition::UpdatePrice(update_price) => {
                Ok(DocumentUpdatePriceTransitionWASM::from(update_price))
            }
            _ => Err(JsValue::undefined()),
        }
    }

    #[wasm_bindgen(setter = "dataContractId")]
    pub fn set_data_contract_id(&mut self, js_data_contract_id: &JsValue) -> Result<(), JsValue> {
        Ok(self
            .0
            .set_data_contract_id(IdentifierWASM::try_from(js_data_contract_id)?.into()))
    }

    #[wasm_bindgen(setter = "revision")]
    pub fn set_revision(&mut self, revision: Revision) {
        self.0.set_revision(revision)
    }

    #[wasm_bindgen(setter = "identityContractNonce")]
    pub fn set_identity_contract_nonce(&mut self, identity_contract_nonce: IdentityNonce) {
        self.0.set_identity_contract_nonce(identity_contract_nonce)
    }
}

impl DocumentTransitionWASM {
    pub fn rs_get_data_contract_id(&self) -> Identifier {
        self.0.data_contract_id()
    }

    pub fn rs_get_id(&self) -> Identifier {
        self.0.get_id()
    }

    pub fn rs_get_entropy(&self) -> Option<Vec<u8>> {
        self.0.entropy()
    }

    pub fn rs_get_revision(&self) -> Option<Revision> {
        self.0.revision()
    }

    pub fn rs_get_identity_contract_nonce(&self) -> IdentityNonce {
        self.0.identity_contract_nonce()
    }
}
