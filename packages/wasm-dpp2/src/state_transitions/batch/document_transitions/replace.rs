use crate::data_contract::document::DocumentWasm;
use crate::error::WasmDppResult;
use crate::impl_wasm_type_info;
use crate::serialization;
use crate::state_transitions::batch::document_base_transition::DocumentBaseTransitionWasm;
use crate::state_transitions::batch::document_transition::DocumentTransitionWasm;
use crate::state_transitions::batch::generators::generate_replace_transition;
use crate::state_transitions::batch::token_payment_info::TokenPaymentInfoWasm;
use crate::utils::ToSerdeJSONExt;
use dpp::prelude::{IdentityNonce, Revision};
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
use dpp::state_transition::batch_transition::document_replace_transition::v0::v0_methods::DocumentReplaceTransitionV0Methods;
use dpp::state_transition::batch_transition::DocumentReplaceTransition;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "Record<string, unknown>")]
    pub type DocumentTransitionDataJs;
}

#[wasm_bindgen(js_name = "DocumentReplaceTransition")]
pub struct DocumentReplaceTransitionWasm(DocumentReplaceTransition);

impl From<DocumentReplaceTransition> for DocumentReplaceTransitionWasm {
    fn from(document_replace: DocumentReplaceTransition) -> Self {
        DocumentReplaceTransitionWasm(document_replace)
    }
}

impl From<DocumentReplaceTransitionWasm> for DocumentReplaceTransition {
    fn from(document_replace: DocumentReplaceTransitionWasm) -> Self {
        document_replace.0
    }
}

#[wasm_bindgen(js_class = DocumentReplaceTransition)]
impl DocumentReplaceTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        document: &DocumentWasm,
        #[wasm_bindgen(js_name = "identityContractNonce")] identity_contract_nonce: IdentityNonce,
        #[wasm_bindgen(js_name = "tokenPaymentInfo")] token_payment_info: Option<
            TokenPaymentInfoWasm,
        >,
    ) -> WasmDppResult<DocumentReplaceTransitionWasm> {
        let rs_update_transition = generate_replace_transition(
            document,
            identity_contract_nonce,
            document.document_type_name().to_string(),
            token_payment_info,
        );

        Ok(DocumentReplaceTransitionWasm(rs_update_transition))
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

    #[wasm_bindgen(getter = "revision")]
    pub fn revision(&self) -> Revision {
        self.0.revision()
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
    ) -> WasmDppResult<DocumentReplaceTransitionWasm> {
        transition.replace_transition()
    }
}

impl_wasm_type_info!(DocumentReplaceTransitionWasm, DocumentReplaceTransition);
