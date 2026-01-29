use crate::state_transitions::batch::document_transitions::create::DocumentCreateTransitionWasm;
use crate::state_transitions::batch::document_transitions::delete::DocumentDeleteTransitionWasm;
use crate::state_transitions::batch::document_transitions::purchase::DocumentPurchaseTransitionWasm;
use crate::state_transitions::batch::document_transitions::replace::DocumentReplaceTransitionWasm;
use crate::state_transitions::batch::document_transitions::transfer::DocumentTransferTransitionWasm;
use crate::state_transitions::batch::document_transitions::update_price::DocumentUpdatePriceTransitionWasm;
use crate::enums::batch::batch_enum::BatchTypeWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_try_from_js_value;
use crate::impl_wasm_type_info;
use dpp::prelude::{IdentityNonce, Revision};
use dpp::state_transition::batch_transition::batched_transition::document_transition::{
    DocumentTransition, DocumentTransitionV0Methods,
};
use dpp::state_transition::batch_transition::batched_transition::document_transition_action_type::{
    DocumentTransitionActionType, DocumentTransitionActionTypeGetter,
};
use crate::utils::try_to_u64;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

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
    #[wasm_bindgen(getter = "actionType")]
    pub fn action_type(&self) -> String {
        BatchTypeWasm::from(self.0.action_type()).into()
    }

    #[wasm_bindgen(getter = "actionTypeNumber")]
    pub fn action_type_number(&self) -> u8 {
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
    pub fn data_contract_id(&self) -> IdentifierWasm {
        self.0.data_contract_id().into()
    }

    #[wasm_bindgen(getter = "id")]
    pub fn id(&self) -> IdentifierWasm {
        self.0.get_id().into()
    }

    #[wasm_bindgen(getter = "documentTypeName")]
    pub fn document_type_name(&self) -> String {
        self.0.document_type_name().clone()
    }

    #[wasm_bindgen(getter = "identityContractNonce")]
    pub fn identity_contract_nonce(&self) -> IdentityNonce {
        self.0.identity_contract_nonce()
    }

    #[wasm_bindgen(getter = "revision")]
    pub fn revision(&self) -> Option<Revision> {
        self.0.revision()
    }

    #[wasm_bindgen(getter = "entropy")]
    pub fn entropy(&self) -> Option<Vec<u8>> {
        self.0.entropy()
    }

    #[wasm_bindgen(getter = "createTransition")]
    pub fn create_transition(&self) -> WasmDppResult<DocumentCreateTransitionWasm> {
        match self.0.clone() {
            DocumentTransition::Create(create) => Ok(DocumentCreateTransitionWasm::from(create)),
            _ => Err(WasmDppError::invalid_argument(
                "Document transition is not a create transition",
            )),
        }
    }

    #[wasm_bindgen(getter = "replaceTransition")]
    pub fn replace_transition(&self) -> WasmDppResult<DocumentReplaceTransitionWasm> {
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
    pub fn delete_transition(&self) -> WasmDppResult<DocumentDeleteTransitionWasm> {
        match self.0.clone() {
            DocumentTransition::Delete(delete) => Ok(DocumentDeleteTransitionWasm::from(delete)),
            _ => Err(WasmDppError::invalid_argument(
                "Document transition is not a delete transition",
            )),
        }
    }

    #[wasm_bindgen(getter = "purchaseTransition")]
    pub fn purchase_transition(&self) -> WasmDppResult<DocumentPurchaseTransitionWasm> {
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
    pub fn transfer_transition(&self) -> WasmDppResult<DocumentTransferTransitionWasm> {
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
    pub fn update_price_transition(&self) -> WasmDppResult<DocumentUpdatePriceTransitionWasm> {
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
        data_contract_id: IdentifierLikeJs,
    ) -> WasmDppResult<()> {
        self.0.set_data_contract_id(data_contract_id.try_into()?);
        Ok(())
    }

    #[wasm_bindgen(setter = "revision")]
    pub fn set_revision(&mut self, revision: JsValue) -> WasmDppResult<()> {
        self.0.set_revision(try_to_u64(&revision, "revision")?);
        Ok(())
    }

    #[wasm_bindgen(setter = "identityContractNonce")]
    pub fn set_identity_contract_nonce(
        &mut self,
        #[wasm_bindgen(js_name = "identityContractNonce")] identity_contract_nonce: JsValue,
    ) -> WasmDppResult<()> {
        self.0.set_identity_contract_nonce(try_to_u64(
            &identity_contract_nonce,
            "identityContractNonce",
        )?);
        Ok(())
    }
}

impl_try_from_js_value!(DocumentTransitionWasm, "DocumentTransition");
impl_wasm_type_info!(DocumentTransitionWasm, DocumentTransition);
