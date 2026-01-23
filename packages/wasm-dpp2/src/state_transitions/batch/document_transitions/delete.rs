use crate::data_contract::document::DocumentWasm;
use crate::error::WasmDppResult;
use crate::impl_wasm_type_info;
use crate::state_transitions::batch::document_base_transition::DocumentBaseTransitionWasm;
use crate::state_transitions::batch::document_transition::DocumentTransitionWasm;
use crate::state_transitions::batch::generators::generate_delete_transition;
use crate::state_transitions::batch::token_payment_info::TokenPaymentInfoWasm;
use dpp::prelude::IdentityNonce;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
use dpp::state_transition::batch_transition::DocumentDeleteTransition;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = "DocumentDeleteTransition")]
pub struct DocumentDeleteTransitionWasm(DocumentDeleteTransition);

impl From<DocumentDeleteTransition> for DocumentDeleteTransitionWasm {
    fn from(document_delete_transition: DocumentDeleteTransition) -> Self {
        DocumentDeleteTransitionWasm(document_delete_transition)
    }
}

#[wasm_bindgen(js_class = DocumentDeleteTransition)]
impl DocumentDeleteTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        document: &DocumentWasm,
        #[wasm_bindgen(js_name = "identityContractNonce")] identity_contract_nonce: IdentityNonce,
        #[wasm_bindgen(js_name = "tokenPaymentInfo")] token_payment_info: Option<TokenPaymentInfoWasm>,
    ) -> WasmDppResult<DocumentDeleteTransitionWasm> {
        let rs_delete_transition = generate_delete_transition(
            document,
            identity_contract_nonce,
            document.document_type_name().to_string(),
            token_payment_info,
        );

        Ok(DocumentDeleteTransitionWasm(rs_delete_transition))
    }

    #[wasm_bindgen(getter = "base")]
    pub fn base(&self) -> DocumentBaseTransitionWasm {
        self.0.base().clone().into()
    }

    #[wasm_bindgen(setter = "base")]
    pub fn set_base(&mut self, base: &DocumentBaseTransitionWasm) {
        self.0.set_base(base.clone().into())
    }

    #[wasm_bindgen(js_name = "toDocumentTransition")]
    pub fn to_document_transition(&self) -> DocumentTransitionWasm {
        let rs_transition = DocumentTransition::from(self.0.clone());

        DocumentTransitionWasm::from(rs_transition)
    }

    #[wasm_bindgen(js_name = "fromDocumentTransition")]
    pub fn from_document_transition(
        transition: DocumentTransitionWasm,
    ) -> WasmDppResult<DocumentDeleteTransitionWasm> {
        transition.delete_transition()
    }
}

impl From<DocumentDeleteTransitionWasm> for DocumentDeleteTransition {
    fn from(document_delete_transition: DocumentDeleteTransitionWasm) -> Self {
        document_delete_transition.0
    }
}

impl_wasm_type_info!(DocumentDeleteTransitionWasm, DocumentDeleteTransition);
