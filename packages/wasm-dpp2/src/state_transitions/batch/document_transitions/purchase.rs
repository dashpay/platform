use crate::state_transitions::batch::document_base_transition::DocumentBaseTransitionWasm;
use crate::state_transitions::batch::document_transition::DocumentTransitionWasm;
use crate::state_transitions::batch::generators::generate_purchase_transition;
use crate::state_transitions::batch::token_payment_info::TokenPaymentInfoWasm;
use crate::data_contract::document::DocumentWasm;
use crate::error::WasmDppResult;
use crate::utils::IntoWasm;
use dpp::fee::Credits;
use dpp::prelude::{IdentityNonce, Revision};
use dpp::state_transition::batch_transition::batched_transition::document_purchase_transition::v0::v0_methods::DocumentPurchaseTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::batched_transition::DocumentPurchaseTransition;
use dpp::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

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
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "DocumentPurchaseTransition".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "DocumentPurchaseTransition".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        document: &DocumentWasm,
        identity_contract_nonce: IdentityNonce,
        amount: Credits,
        js_token_payment_info: &JsValue,
    ) -> WasmDppResult<DocumentPurchaseTransitionWasm> {
        let token_payment_info =
            match js_token_payment_info.is_null() | js_token_payment_info.is_undefined() {
                true => None,
                false => Some(
                    js_token_payment_info
                        .to_wasm::<TokenPaymentInfoWasm>("TokenPaymentInfo")?
                        .clone(),
                ),
            };

        let rs_purchase_transition = generate_purchase_transition(
            document.clone(),
            identity_contract_nonce,
            document.get_document_type_name().to_string(),
            amount,
            token_payment_info,
        );

        Ok(DocumentPurchaseTransitionWasm(rs_purchase_transition))
    }

    #[wasm_bindgen(getter = "base")]
    pub fn get_base(&self) -> DocumentBaseTransitionWasm {
        self.0.base().clone().into()
    }

    #[wasm_bindgen(getter = "price")]
    pub fn get_price(&self) -> Credits {
        self.0.price()
    }

    #[wasm_bindgen(getter = "revision")]
    pub fn get_revision(&self) -> Revision {
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
        js_transition: DocumentTransitionWasm,
    ) -> WasmDppResult<DocumentPurchaseTransitionWasm> {
        js_transition.get_purchase_transition()
    }
}
