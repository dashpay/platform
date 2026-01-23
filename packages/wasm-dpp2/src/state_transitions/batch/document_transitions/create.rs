use crate::data_contract::document::DocumentWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::impl_wasm_type_info;
use crate::serialization;
use crate::state_transitions::batch::document_base_transition::DocumentBaseTransitionWasm;
use crate::state_transitions::batch::document_transition::DocumentTransitionWasm;
use crate::state_transitions::batch::generators::generate_create_transition;
use crate::state_transitions::batch::prefunded_voting_balance::PrefundedVotingBalanceWasm;
use crate::state_transitions::batch::token_payment_info::TokenPaymentInfoWasm;
use crate::utils::ToSerdeJSONExt;
use dpp::prelude::IdentityNonce;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
use dpp::state_transition::batch_transition::document_create_transition::v0::v0_methods::DocumentCreateTransitionV0Methods;
use dpp::state_transition::batch_transition::DocumentCreateTransition;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "Record<string, unknown>")]
    pub type DocumentTransitionDataJs;
}

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
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        document: &DocumentWasm,
        #[wasm_bindgen(js_name = "identityContractNonce")] identity_contract_nonce: IdentityNonce,
        #[wasm_bindgen(js_name = "prefundedVotingBalance")] prefunded_voting_balance: Option<PrefundedVotingBalanceWasm>,
        #[wasm_bindgen(js_name = "tokenPaymentInfo")] token_payment_info: Option<TokenPaymentInfoWasm>,
    ) -> WasmDppResult<DocumentCreateTransitionWasm> {
        let rs_create_transition = generate_create_transition(
            document,
            identity_contract_nonce,
            document.document_type_name().to_string(),
            prefunded_voting_balance,
            token_payment_info,
        );

        Ok(DocumentCreateTransitionWasm(rs_create_transition))
    }

    #[wasm_bindgen(getter = "data")]
    pub fn data(&self) -> WasmDppResult<DocumentTransitionDataJs> {
        let js_value = serialization::to_object(self.0.data())?;
        Ok(js_value.into())
    }

    #[wasm_bindgen(getter = "base")]
    pub fn base(&self) -> DocumentBaseTransitionWasm {
        self.0.base().clone().into()
    }

    #[wasm_bindgen(getter = "entropy")]
    pub fn entropy(&self) -> Vec<u8> {
        self.0.entropy().to_vec()
    }

    #[wasm_bindgen(setter = "data")]
    pub fn set_data(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "Record<string, unknown>")] data: JsValue,
    ) -> WasmDppResult<()> {
        let data = data.with_serde_to_platform_value_map()?;

        self.0.set_data(data);
        Ok(())
    }

    #[wasm_bindgen(setter = "base")]
    pub fn set_base(&mut self, base: &DocumentBaseTransitionWasm) {
        self.0.set_base(base.clone().into())
    }

    #[wasm_bindgen(setter = "entropy")]
    pub fn set_entropy(&mut self, entropy: Vec<u8>) -> WasmDppResult<()> {
        if entropy.len() != 32 {
            return Err(WasmDppError::invalid_argument(format!(
                "Entropy must be exactly 32 bytes, got {}",
                entropy.len()
            )));
        }
        let mut entropy_bytes = [0u8; 32];
        entropy_bytes.copy_from_slice(&entropy);

        self.0.set_entropy(entropy_bytes);
        Ok(())
    }

    #[wasm_bindgen(getter = "prefundedVotingBalance")]
    pub fn prefunded_voting_balance(&self) -> Option<PrefundedVotingBalanceWasm> {
        let rs_balance = self.0.prefunded_voting_balance();

        rs_balance.as_ref().map(|balance| balance.clone().into())
    }

    #[wasm_bindgen(setter = "prefundedVotingBalance")]
    pub fn set_prefunded_voting_balance(
        &mut self,
        #[wasm_bindgen(js_name = "prefundedVotingBalance")] prefunded_voting_balance: &PrefundedVotingBalanceWasm,
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
        transition: &DocumentTransitionWasm,
    ) -> WasmDppResult<DocumentCreateTransitionWasm> {
        transition.create_transition()
    }
}

impl_wasm_type_info!(DocumentCreateTransitionWasm, DocumentCreateTransition);
