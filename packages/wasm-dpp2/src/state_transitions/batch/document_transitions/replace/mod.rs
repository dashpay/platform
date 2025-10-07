use crate::state_transitions::batch::document_base_transition::DocumentBaseTransitionWasm;
use crate::state_transitions::batch::document_transition::DocumentTransitionWasm;
use crate::state_transitions::batch::generators::generate_replace_transition;
use crate::state_transitions::batch::token_payment_info::TokenPaymentInfoWasm;
use crate::data_contract::document::DocumentWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::utils::{IntoWasm, ToSerdeJSONExt};
use dpp::dashcore::hashes::serde::Serialize;
use dpp::prelude::{IdentityNonce, Revision};
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
use dpp::state_transition::batch_transition::document_replace_transition::v0::v0_methods::DocumentReplaceTransitionV0Methods;
use dpp::state_transition::batch_transition::DocumentReplaceTransition;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

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
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "DocumentReplaceTransition".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "DocumentReplaceTransition".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        document: &DocumentWasm,
        identity_contract_nonce: IdentityNonce,
        js_token_payment_info: &JsValue,
    ) -> WasmDppResult<DocumentReplaceTransitionWasm> {
        let token_payment_info =
            match js_token_payment_info.is_null() | js_token_payment_info.is_undefined() {
                true => None,
                false => Some(
                    js_token_payment_info
                        .to_wasm::<TokenPaymentInfoWasm>("TokenPaymentInfo")?
                        .clone(),
                ),
            };

        let rs_update_transition = generate_replace_transition(
            document.clone(),
            identity_contract_nonce,
            document.get_document_type_name().to_string(),
            token_payment_info,
        );

        Ok(DocumentReplaceTransitionWasm(rs_update_transition))
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

    #[wasm_bindgen(getter = "revision")]
    pub fn get_revision(&self) -> Revision {
        self.0.revision()
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

    #[wasm_bindgen(setter = "revision")]
    pub fn set_revision(&mut self, revision: Revision) {
        self.0.set_revision(revision);
    }

    #[wasm_bindgen(js_name = "toDocumentTransition")]
    pub fn to_document_transition(&self) -> DocumentTransitionWasm {
        let rs_transition = DocumentTransition::from(self.0.clone());

        DocumentTransitionWasm::from(rs_transition)
    }

    #[wasm_bindgen(js_name = "fromDocumentTransition")]
    pub fn from_document_transition(
        js_transition: DocumentTransitionWasm,
    ) -> WasmDppResult<DocumentReplaceTransitionWasm> {
        js_transition.get_replace_transition()
    }
}
