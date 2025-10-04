use dpp::prelude::IdentityNonce;
use dpp::state_transition::batch_transition::batched_transition::document_transfer_transition::v0::v0_methods::DocumentTransferTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::batched_transition::DocumentTransferTransition;
use dpp::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;
use crate::document::DocumentWASM;
use crate::identifier::IdentifierWASM;
use crate::utils::IntoWasm;
use crate::batch::document_base_transition::DocumentBaseTransitionWASM;
use crate::batch::document_transition::DocumentTransitionWASM;
use crate::batch::generators::generate_transfer_transition;
use crate::batch::token_payment_info::TokenPaymentInfoWASM;

#[wasm_bindgen(js_name = "DocumentTransferTransitionWASM")]
pub struct DocumentTransferTransitionWASM(DocumentTransferTransition);

impl From<DocumentTransferTransition> for DocumentTransferTransitionWASM {
    fn from(transition: DocumentTransferTransition) -> Self {
        DocumentTransferTransitionWASM(transition)
    }
}

impl From<DocumentTransferTransitionWASM> for DocumentTransferTransition {
    fn from(transition: DocumentTransferTransitionWASM) -> Self {
        transition.0
    }
}

#[wasm_bindgen]
impl DocumentTransferTransitionWASM {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "DocumentTransferTransitionWASM".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "DocumentTransferTransitionWASM".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        document: &DocumentWASM,
        identity_contract_nonce: IdentityNonce,
        js_recipient_owner_id: &JsValue,
        js_token_payment_info: &JsValue,
    ) -> Result<DocumentTransferTransitionWASM, JsValue> {
        let token_payment_info =
            match js_token_payment_info.is_null() | js_token_payment_info.is_undefined() {
                true => None,
                false => Some(
                    js_token_payment_info
                        .to_wasm::<TokenPaymentInfoWASM>("TokenPaymentInfoWASM")?
                        .clone(),
                ),
            };

        let rs_transfer_transition = generate_transfer_transition(
            document.clone(),
            identity_contract_nonce,
            document.get_document_type_name().to_string(),
            IdentifierWASM::try_from(js_recipient_owner_id)?.into(),
            token_payment_info,
        );

        Ok(DocumentTransferTransitionWASM(rs_transfer_transition))
    }

    #[wasm_bindgen(getter = "base")]
    pub fn get_base(&self) -> DocumentBaseTransitionWASM {
        self.0.base().clone().into()
    }

    #[wasm_bindgen(getter = "recipientId")]
    pub fn get_recipient_owner_id(&self) -> IdentifierWASM {
        self.0.recipient_owner_id().into()
    }

    #[wasm_bindgen(setter = "base")]
    pub fn set_base(&mut self, base: &DocumentBaseTransitionWASM) {
        self.0.set_base(base.clone().into())
    }

    #[wasm_bindgen(setter = "recipientId")]
    pub fn set_recipient_owner_id(
        &mut self,
        js_recipient_owner_id: &JsValue,
    ) -> Result<(), JsValue> {
        self.0
            .set_recipient_owner_id(IdentifierWASM::try_from(js_recipient_owner_id)?.into());
        Ok(())
    }

    #[wasm_bindgen(js_name = "toDocumentTransition")]
    pub fn to_document_transition(&self) -> DocumentTransitionWASM {
        let rs_transition = DocumentTransition::from(self.0.clone());

        DocumentTransitionWASM::from(rs_transition)
    }

    #[wasm_bindgen(js_name = "fromDocumentTransition")]
    pub fn from_document_transition(
        js_transition: DocumentTransitionWASM,
    ) -> Result<DocumentTransferTransitionWASM, JsValue> {
        js_transition.get_transfer_transition()
    }
}
