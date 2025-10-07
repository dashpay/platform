use crate::state_transitions::batch::document_base_transition::DocumentBaseTransitionWasm;
use crate::state_transitions::batch::document_transition::DocumentTransitionWasm;
use crate::state_transitions::batch::generators::generate_update_price_transition;
use crate::state_transitions::batch::token_payment_info::TokenPaymentInfoWasm;
use crate::data_contract::document::DocumentWasm;
use crate::error::WasmDppResult;
use crate::utils::IntoWasm;
use dpp::fee::Credits;
use dpp::prelude::IdentityNonce;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::batched_transition::document_update_price_transition::v0::v0_methods::DocumentUpdatePriceTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::DocumentUpdatePriceTransition;
use dpp::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name = "DocumentUpdatePriceTransition")]
pub struct DocumentUpdatePriceTransitionWasm(DocumentUpdatePriceTransition);

impl From<DocumentUpdatePriceTransition> for DocumentUpdatePriceTransitionWasm {
    fn from(document_update_price_transition: DocumentUpdatePriceTransition) -> Self {
        DocumentUpdatePriceTransitionWasm(document_update_price_transition)
    }
}

#[wasm_bindgen(js_class = DocumentUpdatePriceTransition)]
impl DocumentUpdatePriceTransitionWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "DocumentUpdatePriceTransition".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "DocumentUpdatePriceTransition".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        document: &DocumentWasm,
        identity_contract_nonce: IdentityNonce,
        price: Credits,
        js_token_payment_info: &JsValue,
    ) -> WasmDppResult<DocumentUpdatePriceTransitionWasm> {
        let token_payment_info =
            match js_token_payment_info.is_null() | js_token_payment_info.is_undefined() {
                true => None,
                false => Some(
                    js_token_payment_info
                        .to_wasm::<TokenPaymentInfoWasm>("TokenPaymentInfo")?
                        .clone(),
                ),
            };

        let rs_document_update_price_transition = generate_update_price_transition(
            document.clone(),
            identity_contract_nonce,
            document.get_document_type_name().to_string(),
            price,
            token_payment_info,
        );

        Ok(DocumentUpdatePriceTransitionWasm(
            rs_document_update_price_transition,
        ))
    }

    #[wasm_bindgen(getter = "base")]
    pub fn get_base(&self) -> DocumentBaseTransitionWasm {
        self.0.base().clone().into()
    }

    #[wasm_bindgen(getter = "price")]
    pub fn get_price(&self) -> Credits {
        self.0.price()
    }

    #[wasm_bindgen(setter = "base")]
    pub fn set_base(&mut self, base: &DocumentBaseTransitionWasm) {
        self.0.set_base(base.clone().into())
    }

    #[wasm_bindgen(setter = "price")]
    pub fn set_price(&mut self, price: Credits) {
        self.0.set_price(price)
    }

    #[wasm_bindgen(js_name = "toDocumentTransition")]
    pub fn to_document_transition(&self) -> DocumentTransitionWasm {
        let rs_transition = DocumentTransition::from(self.0.clone());

        DocumentTransitionWasm::from(rs_transition)
    }

    #[wasm_bindgen(js_name = "fromDocumentTransition")]
    pub fn from_document_transition(
        js_transition: DocumentTransitionWasm,
    ) -> WasmDppResult<DocumentUpdatePriceTransitionWasm> {
        js_transition.get_update_price_transition()
    }
}
