use crate::data_contract::document::DocumentWasm;
use crate::error::WasmDppResult;
use crate::impl_wasm_type_info;
use crate::state_transitions::batch::document_base_transition::DocumentBaseTransitionWasm;
use crate::state_transitions::batch::document_transition::DocumentTransitionWasm;
use crate::state_transitions::batch::generators::generate_purchase_transition;
use crate::state_transitions::batch::token_payment_info::TokenPaymentInfoWasm;
use dpp::fee::Credits;
use dpp::prelude::{IdentityNonce, Revision};
use dpp::state_transition::batch_transition::batched_transition::document_purchase_transition::v0::v0_methods::DocumentPurchaseTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::batched_transition::DocumentPurchaseTransition;
use dpp::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = "DocumentPurchaseTransition")]
pub struct DocumentPurchaseTransitionWasm(DocumentPurchaseTransition);

impl From<DocumentPurchaseTransitionWasm> for DocumentPurchaseTransition {
    fn from(transition: DocumentPurchaseTransitionWasm) -> Self {
        transition.0
    }
}

impl From<DocumentPurchaseTransition> for DocumentPurchaseTransitionWasm {
    fn from(transition: DocumentPurchaseTransition) -> Self {
        DocumentPurchaseTransitionWasm(transition)
    }
}

#[wasm_bindgen(js_class = DocumentPurchaseTransition)]
impl DocumentPurchaseTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        document: &DocumentWasm,
        #[wasm_bindgen(js_name = "identityContractNonce")] identity_contract_nonce: IdentityNonce,
        amount: Credits,
        #[wasm_bindgen(js_name = "tokenPaymentInfo")] token_payment_info: Option<TokenPaymentInfoWasm>,
    ) -> WasmDppResult<DocumentPurchaseTransitionWasm> {
        let rs_purchase_transition = generate_purchase_transition(
            document,
            identity_contract_nonce,
            document.document_type_name().to_string(),
            amount,
            token_payment_info,
        );

        Ok(DocumentPurchaseTransitionWasm(rs_purchase_transition))
    }

    #[wasm_bindgen(getter = "base")]
    pub fn base(&self) -> DocumentBaseTransitionWasm {
        self.0.base().clone().into()
    }

    #[wasm_bindgen(getter = "price")]
    pub fn price(&self) -> Credits {
        self.0.price()
    }

    #[wasm_bindgen(getter = "revision")]
    pub fn revision(&self) -> Revision {
        self.0.revision()
    }

    #[wasm_bindgen(setter = "base")]
    pub fn set_base(&mut self, base: &DocumentBaseTransitionWasm) {
        self.0.set_base(base.clone().into())
    }

    #[wasm_bindgen(setter = "price")]
    pub fn set_price(&mut self, price: Credits) {
        match self.0 {
            DocumentPurchaseTransition::V0(ref mut v0) => v0.price = price,
        }
    }

    #[wasm_bindgen(setter = "revision")]
    pub fn set_revision(&mut self, revision: Revision) {
        self.0.set_revision(revision);
    }

    #[wasm_bindgen(js_name = "toDocumentTransition")]
    pub fn to_document_transition(&self) -> DocumentTransitionWasm {
        let rs_transition = DocumentTransition::from(self.0.clone());

        DocumentTransitionWasm::from(rs_transition)
    }

    #[wasm_bindgen(js_name = "fromDocumentTransition")]
    pub fn from_document_transition(
        transition: &DocumentTransitionWasm,
    ) -> WasmDppResult<DocumentPurchaseTransitionWasm> {
        transition.purchase_transition()
    }
}

impl_wasm_type_info!(DocumentPurchaseTransitionWasm, DocumentPurchaseTransition);
