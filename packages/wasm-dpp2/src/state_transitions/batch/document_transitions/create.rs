use crate::state_transitions::batch::document_base_transition::DocumentBaseTransitionWasm;
use crate::state_transitions::batch::document_transition::DocumentTransitionWasm;
use crate::state_transitions::batch::generators::generate_create_transition;
use crate::state_transitions::batch::prefunded_voting_balance::PrefundedVotingBalanceWasm;
use crate::state_transitions::batch::token_payment_info::TokenPaymentInfoWasm;
use crate::data_contract::document::DocumentWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::utils::{IntoWasm, ToSerdeJSONExt};
use dpp::dashcore::hashes::serde::Serialize;
use dpp::prelude::IdentityNonce;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
use dpp::state_transition::batch_transition::document_create_transition::v0::v0_methods::DocumentCreateTransitionV0Methods;
use dpp::state_transition::batch_transition::DocumentCreateTransition;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name = "DocumentCreateTransition")]
#[derive(Clone)]
pub struct DocumentCreateTransitionWasm(DocumentCreateTransition);

impl From<DocumentCreateTransitionWasm> for DocumentCreateTransition {
    fn from(transition: DocumentCreateTransitionWasm) -> Self {
        transition.0
    }
}

impl From<DocumentCreateTransition> for DocumentCreateTransitionWasm {
    fn from(transition: DocumentCreateTransition) -> Self {
        DocumentCreateTransitionWasm(transition)
    }
}

#[wasm_bindgen(js_class = DocumentCreateTransition)]
impl DocumentCreateTransitionWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "DocumentCreateTransition".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "DocumentCreateTransition".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        document: &DocumentWasm,
        identity_contract_nonce: IdentityNonce,
        js_prefunded_voting_balance: &JsValue,
        js_token_payment_info: &JsValue,
    ) -> WasmDppResult<DocumentCreateTransitionWasm> {
        let prefunded_voting_balance = match js_prefunded_voting_balance.is_undefined()
            | js_prefunded_voting_balance.is_null()
        {
            true => None,
            false => Some(
                js_prefunded_voting_balance
                    .to_wasm::<PrefundedVotingBalanceWasm>("PrefundedVotingBalance")?
                    .clone(),
            ),
        };

        let token_payment_info =
            match js_token_payment_info.is_null() | js_token_payment_info.is_undefined() {
                true => None,
                false => Some(
                    js_token_payment_info
                        .to_wasm::<TokenPaymentInfoWasm>("TokenPaymentInfo")?
                        .clone(),
                ),
            };

        let rs_create_transition = generate_create_transition(
            document.clone(),
            identity_contract_nonce,
            document.get_document_type_name().to_string(),
            prefunded_voting_balance,
            token_payment_info,
        );

        Ok(DocumentCreateTransitionWasm(rs_create_transition))
    }

    #[wasm_bindgen(getter = "data")]
    pub fn get_data(&self) -> WasmDppResult<JsValue> {
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();

        self.0
            .data()
            .serialize(&serializer)
            .map_err(|err| WasmDppError::serialization(err.to_string()))
    }

    #[wasm_bindgen(getter = "base")]
    pub fn get_base(&self) -> DocumentBaseTransitionWasm {
        self.0.base().clone().into()
    }

    #[wasm_bindgen(getter = "entropy")]
    pub fn get_entropy(&self) -> Vec<u8> {
        self.0.entropy().to_vec()
    }

    #[wasm_bindgen(setter = "data")]
    pub fn set_data(&mut self, js_data: JsValue) -> WasmDppResult<()> {
        let data = js_data.with_serde_to_platform_value_map()?;

        self.0.set_data(data);
        Ok(())
    }

    #[wasm_bindgen(setter = "base")]
    pub fn set_base(&mut self, base: &DocumentBaseTransitionWasm) {
        self.0.set_base(base.clone().into())
    }

    #[wasm_bindgen(setter = "entropy")]
    pub fn set_entropy(&mut self, js_entropy: Vec<u8>) -> WasmDppResult<()> {
        let mut entropy = [0u8; 32];
        let bytes = js_entropy.as_slice();
        let len = bytes.len().min(32);
        entropy[..len].copy_from_slice(&bytes[..len]);

        self.0.set_entropy(entropy);
        Ok(())
    }

    #[wasm_bindgen(getter = "prefundedVotingBalance")]
    pub fn get_prefunded_voting_balance(&self) -> Option<PrefundedVotingBalanceWasm> {
        let rs_balance = self.0.prefunded_voting_balance();

        rs_balance.as_ref().map(|balance| balance.clone().into())
    }

    #[wasm_bindgen(setter = "prefundedVotingBalance")]
    pub fn set_prefunded_voting_balance(
        &mut self,
        prefunded_voting_balance: &PrefundedVotingBalanceWasm,
    ) {
        self.0.set_prefunded_voting_balance(
            prefunded_voting_balance.index_name(),
            prefunded_voting_balance.credits(),
        )
    }

    #[wasm_bindgen(js_name = "clearPrefundedVotingBalance")]
    pub fn clear_prefunded_voting_balance(&mut self) {
        self.0.clear_prefunded_voting_balance()
    }

    #[wasm_bindgen(js_name = "toDocumentTransition")]
    pub fn to_document_transition(&self) -> DocumentTransitionWasm {
        let rs_transition = DocumentTransition::from(self.0.clone());

        DocumentTransitionWasm::from(rs_transition)
    }

    #[wasm_bindgen(js_name = "fromDocumentTransition")]
    pub fn from_document_transition(
        js_transition: DocumentTransitionWasm,
    ) -> WasmDppResult<DocumentCreateTransitionWasm> {
        js_transition.get_create_transition()
    }
}
