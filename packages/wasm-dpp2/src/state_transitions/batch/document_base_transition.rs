use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_wasm_type_info;
use crate::state_transitions::batch::action_fee_agreement::DocumentActionFeeAgreementWasm;
use crate::state_transitions::batch::generators::document_base_transition;
use crate::state_transitions::batch::token_payment_info::TokenPaymentInfoWasm;
use crate::utils::{try_from_options, try_from_options_optional, try_to_u64};
use dpp::data_contract::document_type::action_fees::agreement::DocumentActionFeeAgreement;
use dpp::prelude::IdentityNonce;
use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
use dpp::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use dpp::state_transition::batch_transition::document_base_transition::v1::v1_methods::DocumentBaseTransitionV1Methods;
use dpp::state_transition::batch_transition::document_base_transition::v2::v2_methods::DocumentBaseTransitionV2Methods;
use dpp::tokens::token_payment_info::TokenPaymentInfo;
use serde::Deserialize;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DocumentBaseTransitionOptions {
    identity_contract_nonce: IdentityNonce,
    document_type_name: String,
}

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
export interface DocumentBaseTransitionOptions {
    documentId: IdentifierLike;
    identityContractNonce: bigint;
    documentTypeName: string;
    dataContractId: IdentifierLike;
    tokenPaymentInfo?: TokenPaymentInfo;
    actionFeeAgreement?: DocumentActionFeeAgreement;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "DocumentBaseTransitionOptions")]
    pub type DocumentBaseTransitionOptionsJs;
}

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
        options: DocumentBaseTransitionOptionsJs,
    ) -> WasmDppResult<DocumentBaseTransitionWasm> {
        // Extract documentId (required)
        let document_id: IdentifierWasm = try_from_options(&options, "documentId")?;

        // Extract dataContractId (required)
        let data_contract_id: IdentifierWasm = try_from_options(&options, "dataContractId")?;

        // Extract tokenPaymentInfo (optional)
        let token_payment_info: Option<TokenPaymentInfo> =
            try_from_options_optional::<TokenPaymentInfoWasm>(&options, "tokenPaymentInfo")?
                .map(Into::into);

        // Extract actionFeeAgreement (optional)
        let action_fee_agreement: Option<DocumentActionFeeAgreement> =
            try_from_options_optional::<DocumentActionFeeAgreementWasm>(
                &options,
                "actionFeeAgreement",
            )?
            .map(Into::into);

        // Extract simple fields via serde
        let opts: DocumentBaseTransitionOptions = serde_wasm_bindgen::from_value(options.into())
            .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        Ok(DocumentBaseTransitionWasm(document_base_transition(
            document_id.into(),
            opts.identity_contract_nonce,
            opts.document_type_name,
            data_contract_id.into(),
            token_payment_info,
            action_fee_agreement,
        )))
    }

    #[wasm_bindgen(getter = "id")]
    pub fn id(&self) -> IdentifierWasm {
        self.0.id().into()
    }

    #[wasm_bindgen(getter = "identityContractNonce")]
    pub fn identity_contract_nonce(&self) -> IdentityNonce {
        self.0.identity_contract_nonce()
    }

    #[wasm_bindgen(getter = "dataContractId")]
    pub fn data_contract_id(&self) -> IdentifierWasm {
        self.0.data_contract_id().into()
    }

    #[wasm_bindgen(getter = "documentTypeName")]
    pub fn document_type_name(&self) -> String {
        self.0.document_type_name().to_string()
    }

    #[wasm_bindgen(getter = "tokenPaymentInfo")]
    pub fn token_payment_info(&self) -> Option<TokenPaymentInfoWasm> {
        self.0.token_payment_info().map(|info| info.into())
    }

    /// The action fees this transition agrees to pay. A base older than version 2, which a
    /// transition decoded from the wire can carry, has none.
    #[wasm_bindgen(getter = "actionFeeAgreement")]
    pub fn action_fee_agreement(&self) -> Option<DocumentActionFeeAgreementWasm> {
        self.0
            .action_fee_agreement()
            .map(|agreement| agreement.into())
    }

    #[wasm_bindgen(setter = "id")]
    pub fn set_id(&mut self, id: IdentifierLikeJs) -> WasmDppResult<()> {
        self.0.set_id(id.try_into()?);
        Ok(())
    }

    #[wasm_bindgen(setter = "identityContractNonce")]
    pub fn set_identity_contract_nonce(&mut self, nonce: &js_sys::BigInt) -> WasmDppResult<()> {
        self.0
            .set_identity_contract_nonce(try_to_u64(nonce, "identityContractNonce")?);
        Ok(())
    }

    #[wasm_bindgen(setter = "dataContractId")]
    pub fn set_data_contract_id(
        &mut self,
        data_contract_id: IdentifierLikeJs,
    ) -> WasmDppResult<()> {
        self.0.set_data_contract_id(data_contract_id.try_into()?);
        Ok(())
    }

    #[wasm_bindgen(setter = "documentTypeName")]
    pub fn set_document_type_name(
        &mut self,
        #[wasm_bindgen(js_name = "documentTypeName")] document_type_name: String,
    ) {
        self.0.set_document_type_name(document_type_name)
    }

    #[wasm_bindgen(setter = "tokenPaymentInfo")]
    pub fn set_token_payment_info(
        &mut self,
        #[wasm_bindgen(js_name = "tokenPaymentInfo")] token_payment_info: Option<
            TokenPaymentInfoWasm,
        >,
    ) {
        match token_payment_info {
            Some(info) => self.0.set_token_payment_info(info.into()),
            None => self.0.clear_token_payment_info(),
        }
    }

    /// Sets the action fees this transition agrees to pay, `undefined` clears them.
    ///
    /// Only a base of version 2 carries an agreement, so an older base, which the constructor
    /// builds when it is given none, is rebuilt as version 2 to take one.
    #[wasm_bindgen(setter = "actionFeeAgreement")]
    pub fn set_action_fee_agreement(
        &mut self,
        #[wasm_bindgen(js_name = "actionFeeAgreement")] action_fee_agreement: Option<
            DocumentActionFeeAgreementWasm,
        >,
    ) {
        match action_fee_agreement {
            Some(agreement) => {
                self.0 = document_base_transition(
                    self.0.id(),
                    self.0.identity_contract_nonce(),
                    self.0.document_type_name().clone(),
                    self.0.data_contract_id(),
                    self.0.token_payment_info(),
                    Some(agreement.into()),
                )
            }
            None => self.0.clear_action_fee_agreement(),
        }
    }
}

impl_wasm_type_info!(DocumentBaseTransitionWasm, DocumentBaseTransition);
