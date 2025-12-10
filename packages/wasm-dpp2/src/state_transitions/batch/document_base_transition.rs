use crate::error::WasmDppResult;
use crate::identifier::IdentifierWasm;
use crate::state_transitions::batch::token_payment_info::TokenPaymentInfoWasm;
use crate::utils::IntoWasm;
use dpp::prelude::IdentityNonce;
use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
use dpp::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use dpp::state_transition::batch_transition::document_base_transition::v1::DocumentBaseTransitionV1;
use dpp::state_transition::batch_transition::document_base_transition::v1::v1_methods::DocumentBaseTransitionV1Methods;
use dpp::tokens::token_payment_info::TokenPaymentInfo;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone)]
#[wasm_bindgen(js_name = "DocumentBaseTransition")]
pub struct DocumentBaseTransitionWasm(DocumentBaseTransition);

impl From<DocumentBaseTransition> for DocumentBaseTransitionWasm {
    fn from(v: DocumentBaseTransition) -> Self {
        DocumentBaseTransitionWasm(v)
    }
}

impl From<DocumentBaseTransitionWasm> for DocumentBaseTransition {
    fn from(v: DocumentBaseTransitionWasm) -> Self {
        v.0
    }
}

#[wasm_bindgen(js_class = DocumentBaseTransition)]
impl DocumentBaseTransitionWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "DocumentBaseTransition".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "DocumentBaseTransition".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        js_document_id: &JsValue,
        identity_contract_nonce: IdentityNonce,
        document_type_name: String,
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        js_data_contract_id: &JsValue,
        js_token_payment_info: &JsValue,
    ) -> WasmDppResult<DocumentBaseTransitionWasm> {
        let token_payment_info: Option<TokenPaymentInfo> =
            match js_token_payment_info.is_null() | js_token_payment_info.is_undefined() {
                true => None,
                false => Some(
                    js_token_payment_info
                        .to_wasm::<TokenPaymentInfoWasm>("TokenPaymentInfo")?
                        .clone()
                        .into(),
                ),
            };

        let rs_base_v1 = DocumentBaseTransitionV1 {
            id: IdentifierWasm::try_from(js_document_id)?.into(),
            identity_contract_nonce,
            document_type_name,
            data_contract_id: IdentifierWasm::try_from(js_data_contract_id)?.into(),
            token_payment_info,
        };

        Ok(DocumentBaseTransitionWasm(DocumentBaseTransition::from(
            rs_base_v1,
        )))
    }

    #[wasm_bindgen(getter = "id")]
    pub fn get_id(&self) -> IdentifierWasm {
        self.0.id().into()
    }

    #[wasm_bindgen(getter = "identityContractNonce")]
    pub fn get_identity_contract_nonce(&self) -> IdentityNonce {
        self.0.identity_contract_nonce()
    }

    #[wasm_bindgen(getter = "dataContractId")]
    pub fn get_data_contract_id(&self) -> IdentifierWasm {
        self.0.data_contract_id().into()
    }

    #[wasm_bindgen(getter = "documentTypeName")]
    pub fn get_document_type_name(&self) -> String {
        self.0.document_type_name().to_string()
    }

    #[wasm_bindgen(getter = "tokenPaymentInfo")]
    pub fn get_token_payment_info(&self) -> Option<TokenPaymentInfoWasm> {
        self.0.token_payment_info().map(|info| info.into())
    }

    #[wasm_bindgen(setter = "id")]
    pub fn set_id(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")] js_id: &JsValue,
    ) -> WasmDppResult<()> {
        self.0.set_id(IdentifierWasm::try_from(js_id)?.into());
        Ok(())
    }

    #[wasm_bindgen(setter = "identityContractNonce")]
    pub fn set_identity_contract_nonce(&mut self, nonce: IdentityNonce) {
        self.0.set_identity_contract_nonce(nonce)
    }

    #[wasm_bindgen(setter = "dataContractId")]
    pub fn set_data_contract_id(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "Identifier | Uint8Array | string")]
        js_data_contract_id: &JsValue,
    ) -> WasmDppResult<()> {
        self.0
            .set_data_contract_id(IdentifierWasm::try_from(js_data_contract_id)?.into());
        Ok(())
    }

    #[wasm_bindgen(setter = "documentTypeName")]
    pub fn set_document_type_name(&mut self, document_type_name: String) {
        self.0.set_document_type_name(document_type_name)
    }

    #[wasm_bindgen(setter = "tokenPaymentInfo")]
    pub fn set_token_payment_info(&mut self, token_payment_info: &TokenPaymentInfoWasm) {
        self.0
            .set_token_payment_info(token_payment_info.clone().into())
    }
}
