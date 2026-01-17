use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::impl_wasm_type_info;
use crate::state_transitions::batch::token_payment_info::TokenPaymentInfoWasm;
use crate::utils::IntoWasm;
use dpp::prelude::IdentityNonce;
use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
use dpp::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use dpp::state_transition::batch_transition::document_base_transition::v1::DocumentBaseTransitionV1;
use dpp::state_transition::batch_transition::document_base_transition::v1::v1_methods::DocumentBaseTransitionV1Methods;
use dpp::tokens::token_payment_info::TokenPaymentInfo;
use js_sys::{Object, Reflect};
use serde::Deserialize;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DocumentBaseTransitionOptions {
    identity_contract_nonce: IdentityNonce,
    document_type_name: String,
}

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &'static str = r#"
export interface DocumentBaseTransitionOptions {
    documentId: IdentifierLike;
    identityContractNonce: bigint;
    documentTypeName: string;
    dataContractId: IdentifierLike;
    tokenPaymentInfo?: TokenPaymentInfo;
}
"#;

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
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        #[wasm_bindgen(unchecked_param_type = "DocumentBaseTransitionOptions")] options: JsValue,
    ) -> WasmDppResult<DocumentBaseTransitionWasm> {
        let object = Object::from(options.clone());

        // Extract documentId (required)
        let js_document_id = Reflect::get(&object, &JsValue::from_str("documentId"))
            .map_err(|e| WasmDppError::invalid_argument(format!("Missing documentId: {:?}", e)))?;
        let document_id = IdentifierWasm::try_from(&js_document_id)?.into();

        // Extract dataContractId (required)
        let js_data_contract_id = Reflect::get(&object, &JsValue::from_str("dataContractId"))
            .map_err(|e| WasmDppError::invalid_argument(format!("Missing dataContractId: {:?}", e)))?;
        let data_contract_id = IdentifierWasm::try_from(&js_data_contract_id)?.into();

        // Extract tokenPaymentInfo (optional)
        let js_token_payment_info = Reflect::get(&object, &JsValue::from_str("tokenPaymentInfo"))
            .unwrap_or(JsValue::UNDEFINED);
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

        // Extract simple fields via serde
        let opts: DocumentBaseTransitionOptions = serde_wasm_bindgen::from_value(options)
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        let rs_base_v1 = DocumentBaseTransitionV1 {
            id: document_id,
            identity_contract_nonce: opts.identity_contract_nonce,
            document_type_name: opts.document_type_name,
            data_contract_id,
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
        #[wasm_bindgen(unchecked_param_type = "IdentifierLike")] js_id: &JsValue,
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
        #[wasm_bindgen(unchecked_param_type = "IdentifierLike")]
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

impl_wasm_type_info!(DocumentBaseTransitionWasm, DocumentBaseTransition);
