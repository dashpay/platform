use std::sync::Arc;

use dpp::identifier::Identifier;
use dpp::state_transition::documents_batch_transition::token_issuance_transition::v0::TokenIssuanceTransitionV0;
use dpp::ProtocolError;

use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::document::documents_batch::document_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionV0};
use crate::state_transition_action::document::documents_batch::document_transition::token_issuance_transition_action::v0::TokenIssuanceTransitionActionV0;

impl TokenIssuanceTransitionActionV0 {
    /// Attempt to convert a `TokenIssuanceTransitionV0` into a `TokenIssuanceTransitionActionV0` using a data contract lookup function.
    ///
    /// # Arguments
    ///
    /// * `value` - A `TokenIssuanceTransitionV0` from which to derive the action
    /// * `get_data_contract` - A closure that, given a `data_contract_id`, returns an `Arc<DataContractFetchInfo>`
    ///
    /// # Returns
    ///
    /// * `Result<TokenIssuanceTransitionActionV0, ProtocolError>` - A `TokenIssuanceTransitionActionV0` if successful, else `ProtocolError`.
    pub fn try_from_token_issuance_transition_with_contract_lookup(
        value: TokenIssuanceTransitionV0,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        let TokenIssuanceTransitionV0 { base, amount } = value;

        let base_action = TokenBaseTransitionAction::try_from_base_transition_with_contract_lookup(
            base,
            get_data_contract,
        )?;

        Ok(TokenIssuanceTransitionActionV0 {
            base: base_action,
            issuance_amount: amount,
        })
    }

    /// Attempt to convert a borrowed `TokenIssuanceTransitionV0` into a `TokenIssuanceTransitionActionV0` using a data contract lookup function.
    ///
    /// # Arguments
    ///
    /// * `value` - A reference to a `TokenIssuanceTransitionV0` from which to derive the action
    /// * `get_data_contract` - A closure that, given a `data_contract_id`, returns an `Arc<DataContractFetchInfo>`
    ///
    /// # Returns
    ///
    /// * `Result<TokenIssuanceTransitionActionV0, ProtocolError>` - A `TokenIssuanceTransitionActionV0` if successful, else `ProtocolError`.
    pub fn try_from_borrowed_token_issuance_transition_with_contract_lookup(
        value: &TokenIssuanceTransitionV0,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        let TokenIssuanceTransitionV0 { base, amount } = value;

        let base_action =
            TokenBaseTransitionAction::try_from_borrowed_base_transition_with_contract_lookup(
                base,
                get_data_contract,
            )?;

        Ok(TokenIssuanceTransitionActionV0 {
            base: base_action,
            issuance_amount: *amount,
        })
    }
}