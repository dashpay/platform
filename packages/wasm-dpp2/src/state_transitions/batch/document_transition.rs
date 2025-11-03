use crate::state_transitions::batch::document_transitions::create::DocumentCreateTransitionWasm;
use crate::state_transitions::batch::document_transitions::delete::DocumentDeleteTransitionWasm;
use crate::state_transitions::batch::document_transitions::purchase::DocumentPurchaseTransitionWasm;
use crate::state_transitions::batch::document_transitions::replace::DocumentReplaceTransitionWasm;
use crate::state_transitions::batch::document_transitions::transfer::DocumentTransferTransitionWasm;
use crate::state_transitions::batch::document_transitions::update_price::DocumentUpdatePriceTransitionWasm;
use crate::enums::batch::batch_enum::BatchTypeWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use dpp::prelude::{Identifier, IdentityNonce, Revision};
use dpp::state_transition::batch_transition::batched_transition::document_transition::{
    DocumentTransition, DocumentTransitionV0Methods,
};
use dpp::state_transition::batch_transition::batched_transition::document_transition_action_type::{
    DocumentTransitionActionType, DocumentTransitionActionTypeGetter,
};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[derive(Clone)]
#[wasm_bindgen(js_name = "DocumentTransition")]
pub struct DocumentTransitionWasm(DocumentTransition);

impl From<DocumentTransition> for DocumentTransitionWasm {
    fn from(transition: DocumentTransition) -> Self {
        DocumentTransitionWasm(transition)
    }
}

impl From<DocumentTransitionWasm> for DocumentTransition {
    fn from(transition: DocumentTransitionWasm) -> Self {
        transition.0
    }
}

#[wasm_bindgen(js_class = DocumentTransition)]
impl DocumentTransitionWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "DocumentTransition".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "DocumentTransition".to_string()
    }

    #[wasm_bindgen(getter = "actionType")]
    pub fn get_action_type(&self) -> String {
        BatchTypeWasm::from(self.0.action_type()).into()
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
    pub fn get_data_contract_id(&self) -> IdentifierWasm {
        self.0.data_contract_id().into()
    }

    #[wasm_bindgen(getter = "id")]
    pub fn get_id(&self) -> IdentifierWasm {
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
    pub fn get_create_transition(&self) -> WasmDppResult<DocumentCreateTransitionWasm> {
        match self.0.clone() {
            DocumentTransition::Create(create) => Ok(DocumentCreateTransitionWasm::from(create)),
            _ => Err(WasmDppError::invalid_argument(
                "Document transition is not a create transition",
            )),
        }
    }

    #[wasm_bindgen(getter = "replaceTransition")]
    pub fn get_replace_transition(&self) -> WasmDppResult<DocumentReplaceTransitionWasm> {
        match self.0.clone() {
            DocumentTransition::Replace(replace) => {
                Ok(DocumentReplaceTransitionWasm::from(replace))
            }
            _ => Err(WasmDppError::invalid_argument(
                "Document transition is not a replace transition",
            )),
        }
    }

    #[wasm_bindgen(getter = "deleteTransition")]
    pub fn get_delete_transition(&self) -> WasmDppResult<DocumentDeleteTransitionWasm> {
        match self.0.clone() {
            DocumentTransition::Delete(delete) => Ok(DocumentDeleteTransitionWasm::from(delete)),
            _ => Err(WasmDppError::invalid_argument(
                "Document transition is not a delete transition",
            )),
        }
    }

    #[wasm_bindgen(getter = "purchaseTransition")]
    pub fn get_purchase_transition(&self) -> WasmDppResult<DocumentPurchaseTransitionWasm> {
        match self.0.clone() {
            DocumentTransition::Purchase(purchase) => {
                Ok(DocumentPurchaseTransitionWasm::from(purchase))
            }
            _ => Err(WasmDppError::invalid_argument(
                "Document transition is not a purchase transition",
            )),
        }
    }

    #[wasm_bindgen(getter = "transferTransition")]
    pub fn get_transfer_transition(&self) -> WasmDppResult<DocumentTransferTransitionWasm> {
        match self.0.clone() {
            DocumentTransition::Transfer(transfer) => {
                Ok(DocumentTransferTransitionWasm::from(transfer))
            }
            _ => Err(WasmDppError::invalid_argument(
                "Document transition is not a transfer transition",
            )),
        }
    }

    #[wasm_bindgen(getter = "updatePriceTransition")]
    pub fn get_update_price_transition(&self) -> WasmDppResult<DocumentUpdatePriceTransitionWasm> {
        match self.0.clone() {
            DocumentTransition::UpdatePrice(update_price) => {
                Ok(DocumentUpdatePriceTransitionWasm::from(update_price))
            }
            _ => Err(WasmDppError::invalid_argument(
                "Document transition is not an update price transition",
            )),
        }
    }

    #[wasm_bindgen(setter = "dataContractId")]
    pub fn set_data_contract_id(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        js_data_contract_id: &JsValue,
    ) -> WasmDppResult<()> {
        self.0
            .set_data_contract_id(IdentifierWasm::try_from(js_data_contract_id)?.into());
        Ok(())
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

impl DocumentTransitionWasm {
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
