use crate::data_contract::document::DocumentWasm;
use crate::error::WasmDppResult;
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_wasm_type_info;
use crate::state_transitions::batch::document_base_transition::DocumentBaseTransitionWasm;
use crate::state_transitions::batch::document_transition::DocumentTransitionWasm;
use crate::state_transitions::batch::generators::generate_transfer_transition;
use crate::state_transitions::batch::token_payment_info::TokenPaymentInfoWasm;
use crate::utils::{try_from_options, try_from_options_optional, try_from_options_with, try_to_u64};
use dpp::prelude::IdentityNonce;
use dpp::state_transition::batch_transition::batched_transition::document_transfer_transition::v0::v0_methods::DocumentTransferTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::batched_transition::DocumentTransferTransition;
use dpp::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const DOCUMENT_TRANSFER_OPTIONS_TS: &str = r#"
export interface DocumentTransferTransitionOptions {
    document: Document;
    identityContractNonce: bigint;
    recipientOwnerId: IdentifierLike;
    tokenPaymentInfo?: TokenPaymentInfo;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "DocumentTransferTransitionOptions")]
    pub type DocumentTransferTransitionOptionsJs;
}

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
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: DocumentTransferTransitionOptionsJs,
    ) -> WasmDppResult<DocumentTransferTransitionWasm> {
        let document: DocumentWasm = try_from_options(&options, "document")?;

        let identity_contract_nonce: IdentityNonce =
            try_from_options_with(&options, "identityContractNonce", |v| {
                try_to_u64(v, "identityContractNonce")
            })?;

        let recipient_owner_id: IdentifierWasm = try_from_options(&options, "recipientOwnerId")?;

        let token_payment_info: Option<TokenPaymentInfoWasm> =
            try_from_options_optional(&options, "tokenPaymentInfo")?;

        let rs_transfer_transition = generate_transfer_transition(
            &document,
            identity_contract_nonce,
            document.document_type_name().to_string(),
            recipient_owner_id.into(),
            token_payment_info,
        );

        Ok(DocumentTransferTransitionWasm(rs_transfer_transition))
    }

    #[wasm_bindgen(getter = "base")]
    pub fn base(&self) -> DocumentBaseTransitionWasm {
        self.0.base().clone().into()
    }

    #[wasm_bindgen(getter = "recipientOwnerId")]
    pub fn recipient_owner_id(&self) -> IdentifierWasm {
        self.0.recipient_owner_id().into()
    }

    #[wasm_bindgen(setter = "base")]
    pub fn set_base(&mut self, base: &DocumentBaseTransitionWasm) {
        self.0.set_base(base.clone().into())
    }

    #[wasm_bindgen(setter = "recipientOwnerId")]
    pub fn set_recipient_owner_id(
        &mut self,
        #[wasm_bindgen(js_name = "recipientOwnerId")] recipient_owner_id: IdentifierLikeJs,
    ) -> WasmDppResult<()> {
        self.0
            .set_recipient_owner_id(recipient_owner_id.try_into()?);
        Ok(())
    }

    #[wasm_bindgen(js_name = "toDocumentTransition")]
    pub fn to_document_transition(&self) -> DocumentTransitionWasm {
        let rs_transition = DocumentTransition::from(self.0.clone());

        DocumentTransitionWasm::from(rs_transition)
    }

    #[wasm_bindgen(js_name = "fromDocumentTransition")]
    pub fn from_document_transition(
        transition: &DocumentTransitionWasm,
    ) -> WasmDppResult<DocumentTransferTransitionWasm> {
        transition.transfer_transition()
    }
}

impl_wasm_type_info!(DocumentTransferTransitionWasm, DocumentTransferTransition);
