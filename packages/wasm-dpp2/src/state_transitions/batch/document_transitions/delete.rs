use crate::state_transitions::batch::document_base_transition::DocumentBaseTransitionWasm;
use crate::state_transitions::batch::document_transition::DocumentTransitionWasm;
use crate::state_transitions::batch::generators::generate_delete_transition;
use crate::state_transitions::batch::token_payment_info::TokenPaymentInfoWasm;
use crate::data_contract::document::DocumentWasm;
use crate::error::WasmDppResult;
use crate::utils::IntoWasm;
use dpp::prelude::IdentityNonce;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
use dpp::state_transition::batch_transition::DocumentDeleteTransition;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name = "DocumentDeleteTransition")]
pub struct DocumentDeleteTransitionWasm(DocumentDeleteTransition);

impl From<DocumentDeleteTransition> for DocumentDeleteTransitionWasm {
    fn from(document_delete_transition: DocumentDeleteTransition) -> Self {
        DocumentDeleteTransitionWasm(document_delete_transition)
    }
}

#[wasm_bindgen(js_class = DocumentDeleteTransition)]
impl DocumentDeleteTransitionWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "DocumentDeleteTransition".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "DocumentDeleteTransition".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        document: &DocumentWasm,
        identity_contract_nonce: IdentityNonce,
        js_token_payment_info: &JsValue,
    ) -> WasmDppResult<DocumentDeleteTransitionWasm> {
        let token_payment_info =
            match js_token_payment_info.is_null() | js_token_payment_info.is_undefined() {
                true => None,
                false => Some(
                    js_token_payment_info
                        .to_wasm::<TokenPaymentInfoWasm>("TokenPaymentInfo")?
                        .clone(),
                ),
            };

        let rs_delete_transition = generate_delete_transition(
            document.clone(),
            identity_contract_nonce,
            document.get_document_type_name().to_string(),
            token_payment_info,
        );

        Ok(DocumentDeleteTransitionWasm(rs_delete_transition))
    }

    #[wasm_bindgen(getter = "base")]
    pub fn get_base(&self) -> DocumentBaseTransitionWasm {
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
        js_transition: DocumentTransitionWasm,
    ) -> WasmDppResult<DocumentDeleteTransitionWasm> {
        js_transition.get_delete_transition()
    }
}

impl From<DocumentDeleteTransitionWasm> for DocumentDeleteTransition {
    fn from(document_delete_transition: DocumentDeleteTransitionWasm) -> Self {
        document_delete_transition.0
    }
}
