use crate::data_contract::document::DocumentWasm;
use crate::data_contract::DataContractWasm;
use crate::error::WasmDppError;
use crate::error::WasmDppResult;
use crate::impl_wasm_type_info;
use crate::serialization;
use crate::state_transitions::batch::action_fee_agreement::DocumentActionFeeAgreementWasm;
use crate::state_transitions::batch::document_base_transition::DocumentBaseTransitionWasm;
use crate::state_transitions::batch::document_transition::DocumentTransitionWasm;
use crate::state_transitions::batch::generators::generate_create_transition;
use crate::state_transitions::batch::prefunded_voting_balance::PrefundedVotingBalanceWasm;
use crate::state_transitions::batch::token_payment_info::TokenPaymentInfoWasm;
use crate::utils::{
    try_from_options_mut, try_from_options_optional, try_from_options_optional_with,
    try_from_options_with, try_to_u64, try_vec_to_fixed_bytes, ToSerdeJSONExt,
};
use crate::version::PlatformVersionWasm;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::methods::{DocumentTypeBasicMethods, DocumentTypeV0Methods};
use dpp::document::DocumentV0Getters;
use dpp::fee::Credits;
use dpp::prelude::{Identifier, IdentityNonce};
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
use dpp::state_transition::batch_transition::document_base_transition::document_base_transition_trait::DocumentBaseTransitionAccessors;
use dpp::state_transition::batch_transition::document_create_transition::v0::v0_methods::DocumentCreateTransitionV0Methods;
use dpp::state_transition::batch_transition::DocumentCreateTransition;
use dpp::version::PlatformVersion;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[wasm_bindgen(typescript_custom_section)]
const DOCUMENT_CREATE_OPTIONS_TS: &str = r#"
export interface DocumentCreateTransitionOptions {
    /**
     * The document to create. Its id is derived here from its entropy and
     * `identityContractNonce` (from protocol version 14 the id of a new
     * document commits to that nonce), replacing whatever id the document
     * carried, and written back onto this object: after construction
     * `document.id` equals `transition.base.id`.
     */
    document: Document;
    identityContractNonce: bigint;
    /**
     * The contested index the document falls under and what it pays into that
     * contest. Required when the document enters a contest, or the create is
     * refused and still pays its fees. From protocol version 14 the amount is
     * the most it pays: it is charged the fund to join and refused when that is
     * more. Before 14 it must be exactly the contest's fund.
     * `sdk.documents.contestFundToJoin(document)` in the SDK returns the one to
     * state now.
     */
    prefundedVotingBalance?: PrefundedVotingBalance;
    /**
     * The contract of `document`. When given and `prefundedVotingBalance` is
     * not, a document that enters a contest states the contest's fund on its
     * contested index, as the Rust builder does: what joining costs while the
     * contest holds fewer than 250 contenders.
     */
    dataContract?: DataContract;
    /**
     * The most, in credits, a contested create pays into its contest (protocol
     * version 14), stated in place of the amount of its prefunded voting
     * balance. Needs `dataContract` or `prefundedVotingBalance` to name the
     * contested index. A document that joins no contest ignores it.
     */
    contestFund?: bigint;
    tokenPaymentInfo?: TokenPaymentInfo;
    actionFeeAgreement?: DocumentActionFeeAgreement;
    /** Platform version the id is derived for (default: latest) */
    platformVersion?: PlatformVersionLike;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "DocumentCreateTransitionOptions")]
    pub type DocumentCreateTransitionOptionsJs;

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
        options: DocumentCreateTransitionOptionsJs,
    ) -> WasmDppResult<DocumentCreateTransitionWasm> {
        let identity_contract_nonce: IdentityNonce =
            try_from_options_with(&options, "identityContractNonce", |v| {
                try_to_u64(v, "identityContractNonce")
            })?;

        let platform_version: PlatformVersion =
            try_from_options_optional::<PlatformVersionWasm>(&options, "platformVersion")?
                .unwrap_or_default()
                .into();

        let prefunded_voting_balance: Option<PrefundedVotingBalanceWasm> =
            try_from_options_optional(&options, "prefundedVotingBalance")?;

        let token_payment_info: Option<TokenPaymentInfoWasm> =
            try_from_options_optional(&options, "tokenPaymentInfo")?;

        let action_fee_agreement: Option<DocumentActionFeeAgreementWasm> =
            try_from_options_optional(&options, "actionFeeAgreement")?;

        let data_contract: Option<DataContractWasm> =
            try_from_options_optional(&options, "dataContract")?;

        let contest_fund: Option<Credits> =
            try_from_options_optional_with(&options, "contestFund", |v| {
                try_to_u64(v, "contestFund")
            })?;

        if contest_fund.is_some() && prefunded_voting_balance.is_none() && data_contract.is_none() {
            return Err(WasmDppError::invalid_argument(
                "contestFund needs dataContract or prefundedVotingBalance to name the contested index",
            ));
        }

        // The id a document carries before its nonce is known is a
        // placeholder: derive the one consensus will recompute, on the
        // caller's own object so `document.id` matches `transition.base.id`,
        // as `DocumentCreateTransitionV0::from_document` does in dpp.
        //
        // Every other property is read above, before this borrow: a JS
        // getter on the options bag could re-enter the same `Document`, and
        // wasm-bindgen reports a second borrow of a mutably borrowed object
        // as an unrecoverable runtime error, not as a result. From here on
        // nothing calls back into JavaScript.
        let mut document = try_from_options_mut::<DocumentWasm>(&options, "document", "Document")?;

        // Resolved before the id is rewritten, so a refused build leaves the document as it was
        let prefunded_voting_balance = match (prefunded_voting_balance, &data_contract) {
            (Some(prefunded_voting_balance), _) => Some(prefunded_voting_balance.into()),
            (None, Some(data_contract)) => {
                contest_fund_of_contract(&document, data_contract, &platform_version)?
            }
            (None, None) => None,
        };
        // Like `StateTransitionCreationOptions::apply_contest_fund` in dpp
        let prefunded_voting_balance = match (prefunded_voting_balance, contest_fund) {
            (Some((index_name, _)), Some(contest_fund)) => Some((index_name, contest_fund)),
            (prefunded_voting_balance, _) => prefunded_voting_balance,
        };

        document.set_id_for_creation(identity_contract_nonce, &platform_version)?;

        let rs_create_transition = generate_create_transition(
            &document,
            identity_contract_nonce,
            document.document_type_name().to_string(),
            prefunded_voting_balance.map(PrefundedVotingBalanceWasm::from),
            token_payment_info,
            action_fee_agreement,
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
        let entropy_bytes: [u8; 32] = try_vec_to_fixed_bytes(entropy, "entropy")?;
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
        #[wasm_bindgen(js_name = "prefundedVotingBalance")]
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
        transition: &DocumentTransitionWasm,
    ) -> WasmDppResult<DocumentCreateTransitionWasm> {
        transition.create_transition()
    }
}

/// The prefunded voting balance a create of `document` states when it enters a contest of
/// `data_contract`: its contested index and the contest's fund, as
/// `DocumentCreateTransitionV0::from_document` in dpp states it, or `None` when it joins none.
/// The contest is resolved with every `generatedFrom` property generated as the platform
/// generates it; the transition keeps the document's own values.
fn contest_fund_of_contract(
    document: &DocumentWasm,
    data_contract: &DataContractWasm,
    platform_version: &PlatformVersion,
) -> WasmDppResult<Option<(String, Credits)>> {
    let data_contract = data_contract.as_ref();
    let document_contract_id: Identifier = document.data_contract_id.into();
    if data_contract.id() != document_contract_id {
        return Err(WasmDppError::invalid_argument(format!(
            "dataContract {} is not the contract of the document, {}",
            data_contract.id(),
            document_contract_id
        )));
    }
    let document_type = data_contract
        .document_type_for_name(&document.document_type_name)
        .map_err(|error| WasmDppError::invalid_argument(error.to_string()))?;
    let mut resolved = document.document.clone();
    document_type.regenerate_generated_properties(resolved.properties_mut(), platform_version)?;
    Ok(document_type.prefunded_voting_balance_for_document(&resolved, platform_version)?)
}

impl_wasm_type_info!(DocumentCreateTransitionWasm, DocumentCreateTransition);
