use crate::data_contract::document::DocumentWasm;
use crate::error::WasmDppResult;
use crate::impl_wasm_type_info;
use crate::state_transitions::batch::document_base_transition::DocumentBaseTransitionWasm;
use crate::state_transitions::batch::document_transition::DocumentTransitionWasm;
use crate::state_transitions::batch::generators::generate_purchase_transition;
use crate::state_transitions::batch::token_payment_info::TokenPaymentInfoWasm;
use crate::utils::{try_from_options, try_from_options_optional, try_from_options_with, try_to_u64};
use dpp::fee::Credits;
use dpp::prelude::{IdentityNonce, Revision};
use dpp::state_transition::batch_transition::batched_transition::document_purchase_transition::v0::v0_methods::DocumentPurchaseTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::batched_transition::DocumentPurchaseTransition;
use dpp::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[wasm_bindgen(typescript_custom_section)]
const DOCUMENT_PURCHASE_OPTIONS_TS: &str = r#"
export interface DocumentPurchaseTransitionOptions {
    document: Document;
    identityContractNonce: bigint;
    amount: bigint;
    tokenPaymentInfo?: TokenPaymentInfo;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "DocumentPurchaseTransitionOptions")]
    pub type DocumentPurchaseTransitionOptionsJs;
}

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
        options: DocumentPurchaseTransitionOptionsJs,
    ) -> WasmDppResult<DocumentPurchaseTransitionWasm> {
        let document: DocumentWasm = try_from_options(&options, "document")?;

        let identity_contract_nonce: IdentityNonce =
            try_from_options_with(&options, "identityContractNonce", |v| {
                try_to_u64(v, "identityContractNonce")
            })?;

        let amount: Credits =
            try_from_options_with(&options, "amount", |v| try_to_u64(v, "amount"))?;

        let token_payment_info: Option<TokenPaymentInfoWasm> =
            try_from_options_optional(&options, "tokenPaymentInfo")?;

        let rs_purchase_transition = generate_purchase_transition(
            &document,
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
    pub fn set_price(&mut self, price: JsValue) -> WasmDppResult<()> {
        use crate::utils::try_to_u64;
        let price = try_to_u64(&price, "price")?;
        match self.0 {
            DocumentPurchaseTransition::V0(ref mut v0) => v0.price = price,
        }
        Ok(())
    }

    #[wasm_bindgen(setter = "revision")]
    pub fn set_revision(&mut self, revision: JsValue) -> WasmDppResult<()> {
        use crate::utils::try_to_u64;
        self.0.set_revision(try_to_u64(&revision, "revision")?);
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
    ) -> WasmDppResult<DocumentPurchaseTransitionWasm> {
        transition.purchase_transition()
    }
}

impl_wasm_type_info!(DocumentPurchaseTransitionWasm, DocumentPurchaseTransition);
