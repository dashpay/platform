use crate::data_contract::document::DocumentWasm;
use crate::error::WasmDppResult;
use crate::impl_wasm_type_info;
use crate::state_transitions::batch::document_base_transition::DocumentBaseTransitionWasm;
use crate::state_transitions::batch::document_transition::DocumentTransitionWasm;
use crate::state_transitions::batch::generators::generate_update_price_transition;
use crate::state_transitions::batch::token_payment_info::TokenPaymentInfoWasm;
use crate::utils::{try_from_options, try_from_options_optional, try_from_options_with, try_to_u64};
use dpp::fee::Credits;
use dpp::prelude::IdentityNonce;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::batched_transition::document_update_price_transition::v0::v0_methods::DocumentUpdatePriceTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::DocumentUpdatePriceTransition;
use dpp::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[wasm_bindgen(typescript_custom_section)]
const DOCUMENT_UPDATE_PRICE_OPTIONS_TS: &str = r#"
export interface DocumentUpdatePriceTransitionOptions {
    document: Document;
    identityContractNonce: bigint;
    price: bigint;
    tokenPaymentInfo?: TokenPaymentInfo;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "DocumentUpdatePriceTransitionOptions")]
    pub type DocumentUpdatePriceTransitionOptionsJs;
}

#[wasm_bindgen(js_name = "DocumentUpdatePriceTransition")]
pub struct DocumentUpdatePriceTransitionWasm(DocumentUpdatePriceTransition);

impl From<DocumentUpdatePriceTransition> for DocumentUpdatePriceTransitionWasm {
    fn from(document_update_price_transition: DocumentUpdatePriceTransition) -> Self {
        DocumentUpdatePriceTransitionWasm(document_update_price_transition)
    }
}

#[wasm_bindgen(js_class = DocumentUpdatePriceTransition)]
impl DocumentUpdatePriceTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: DocumentUpdatePriceTransitionOptionsJs,
    ) -> WasmDppResult<DocumentUpdatePriceTransitionWasm> {
        let document: DocumentWasm = try_from_options(&options, "document")?;

        let identity_contract_nonce: IdentityNonce =
            try_from_options_with(&options, "identityContractNonce", |v| {
                try_to_u64(v, "identityContractNonce")
            })?;

        let price: Credits = try_from_options_with(&options, "price", |v| try_to_u64(v, "price"))?;

        let token_payment_info: Option<TokenPaymentInfoWasm> =
            try_from_options_optional(&options, "tokenPaymentInfo")?;

        let rs_document_update_price_transition = generate_update_price_transition(
            &document,
            identity_contract_nonce,
            document.document_type_name().to_string(),
            price,
            token_payment_info,
        );

        Ok(DocumentUpdatePriceTransitionWasm(
            rs_document_update_price_transition,
        ))
    }

    #[wasm_bindgen(getter = "base")]
    pub fn base(&self) -> DocumentBaseTransitionWasm {
        self.0.base().clone().into()
    }

    #[wasm_bindgen(getter = "price")]
    pub fn price(&self) -> Credits {
        self.0.price()
    }

    #[wasm_bindgen(setter = "base")]
    pub fn set_base(&mut self, base: &DocumentBaseTransitionWasm) {
        self.0.set_base(base.clone().into())
    }

    #[wasm_bindgen(setter = "price")]
    pub fn set_price(&mut self, price: JsValue) -> WasmDppResult<()> {
        use crate::utils::try_to_u64;
        self.0.set_price(try_to_u64(&price, "price")?);
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
    ) -> WasmDppResult<DocumentUpdatePriceTransitionWasm> {
        transition.update_price_transition()
    }
}

impl_wasm_type_info!(
    DocumentUpdatePriceTransitionWasm,
    DocumentUpdatePriceTransition
);
