use crate::state_transitions::batch::document_base_transition::DocumentBaseTransitionWasm;
use crate::state_transitions::batch::document_transition::DocumentTransitionWasm;
use crate::state_transitions::batch::generators::generate_transfer_transition;
use crate::state_transitions::batch::token_payment_info::TokenPaymentInfoWasm;
use crate::data_contract::document::DocumentWasm;
use crate::error::WasmDppResult;
use crate::identifier::IdentifierWasm;
use crate::utils::IntoWasm;
use dpp::prelude::IdentityNonce;
use dpp::state_transition::batch_transition::batched_transition::document_transfer_transition::v0::v0_methods::DocumentTransferTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::batched_transition::DocumentTransferTransition;
use dpp::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name = "DocumentTransferTransition")]
pub struct DocumentTransferTransitionWasm(DocumentTransferTransition);

impl From<DocumentTransferTransition> for DocumentTransferTransitionWasm {
    fn from(transition: DocumentTransferTransition) -> Self {
        DocumentTransferTransitionWasm(transition)
    }
}

impl From<DocumentTransferTransitionWasm> for DocumentTransferTransition {
    fn from(transition: DocumentTransferTransitionWasm) -> Self {
        transition.0
    }
}

#[wasm_bindgen(js_class = DocumentTransferTransition)]
impl DocumentTransferTransitionWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "DocumentTransferTransition".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "DocumentTransferTransition".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        document: &DocumentWasm,
        identity_contract_nonce: IdentityNonce,
        js_recipient_owner_id: &JsValue,
        js_token_payment_info: &JsValue,
    ) -> WasmDppResult<DocumentTransferTransitionWasm> {
        let token_payment_info =
            match js_token_payment_info.is_null() | js_token_payment_info.is_undefined() {
                true => None,
                false => Some(
                    js_token_payment_info
                        .to_wasm::<TokenPaymentInfoWasm>("TokenPaymentInfo")?
                        .clone(),
                ),
            };

        let rs_transfer_transition = generate_transfer_transition(
            document.clone(),
            identity_contract_nonce,
            document.get_document_type_name().to_string(),
            IdentifierWasm::try_from(js_recipient_owner_id)?.into(),
            token_payment_info,
        );

        Ok(DocumentTransferTransitionWasm(rs_transfer_transition))
    }

    #[wasm_bindgen(getter = "base")]
    pub fn get_base(&self) -> DocumentBaseTransitionWasm {
        self.0.base().clone().into()
    }

    #[wasm_bindgen(getter = "recipientId")]
    pub fn get_recipient_owner_id(&self) -> IdentifierWasm {
        self.0.recipient_owner_id().into()
    }

    #[wasm_bindgen(setter = "base")]
    pub fn set_base(&mut self, base: &DocumentBaseTransitionWasm) {
        self.0.set_base(base.clone().into())
    }

    #[wasm_bindgen(setter = "recipientId")]
    pub fn set_recipient_owner_id(&mut self, js_recipient_owner_id: &JsValue) -> WasmDppResult<()> {
        self.0
            .set_recipient_owner_id(IdentifierWasm::try_from(js_recipient_owner_id)?.into());
        Ok(())
    }

    #[wasm_bindgen(js_name = "toDocumentTransition")]
    pub fn to_document_transition(&self) -> DocumentTransitionWasm {
        let rs_transition = DocumentTransition::from(self.0.clone());

        DocumentTransitionWasm::from(rs_transition)
    }

    #[wasm_bindgen(js_name = "fromDocumentTransition")]
    pub fn from_document_transition(
        js_transition: DocumentTransitionWasm,
    ) -> WasmDppResult<DocumentTransferTransitionWasm> {
        js_transition.get_transfer_transition()
    }
}
