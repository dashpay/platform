use crate::batch::document_base_transition::DocumentBaseTransitionWASM;
use crate::batch::document_transition::DocumentTransitionWASM;
use crate::batch::generators::generate_delete_transition;
use dpp::prelude::IdentityNonce;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
use dpp::state_transition::batch_transition::DocumentDeleteTransition;
use crate::document::DocumentWASM;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;
use crate::utils::IntoWasm;
use crate::batch::token_payment_info::TokenPaymentInfoWASM;

#[wasm_bindgen(js_name = "DocumentDeleteTransition")]
pub struct DocumentDeleteTransitionWASM(DocumentDeleteTransition);

impl From<DocumentDeleteTransition> for DocumentDeleteTransitionWASM {
    fn from(document_delete_transition: DocumentDeleteTransition) -> Self {
        DocumentDeleteTransitionWASM(document_delete_transition)
    }
}

#[wasm_bindgen(js_class = DocumentDeleteTransition)]
impl DocumentDeleteTransitionWASM {
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
        document: &DocumentWASM,
        identity_contract_nonce: IdentityNonce,
        js_token_payment_info: &JsValue,
    ) -> Result<DocumentDeleteTransitionWASM, JsValue> {
        let token_payment_info =
            match js_token_payment_info.is_null() | js_token_payment_info.is_undefined() {
                true => None,
                false => Some(
                    js_token_payment_info
                        .to_wasm::<TokenPaymentInfoWASM>("TokenPaymentInfo")?
                        .clone(),
                ),
            };

        let rs_delete_transition = generate_delete_transition(
            document.clone(),
            identity_contract_nonce,
            document.get_document_type_name().to_string(),
            token_payment_info,
        );

        Ok(DocumentDeleteTransitionWASM(rs_delete_transition))
    }

    #[wasm_bindgen(getter = "base")]
    pub fn get_base(&self) -> DocumentBaseTransitionWASM {
        self.0.base().clone().into()
    }

    #[wasm_bindgen(setter = "base")]
    pub fn set_base(&mut self, base: &DocumentBaseTransitionWASM) {
        self.0.set_base(base.clone().into())
    }

    #[wasm_bindgen(js_name = "toDocumentTransition")]
    pub fn to_document_transition(&self) -> DocumentTransitionWASM {
        let rs_transition = DocumentTransition::from(self.0.clone());

        DocumentTransitionWASM::from(rs_transition)
    }

    #[wasm_bindgen(js_name = "fromDocumentTransition")]
    pub fn from_document_transition(
        js_transition: DocumentTransitionWASM,
    ) -> Result<DocumentDeleteTransitionWASM, JsValue> {
        js_transition.get_delete_transition()
    }
}

impl From<DocumentDeleteTransitionWASM> for DocumentDeleteTransition {
    fn from(document_delete_transition: DocumentDeleteTransitionWASM) -> Self {
        document_delete_transition.0
    }
}
