use std::sync::Arc;

use dpp::platform_value::Identifier;
use dpp::ProtocolError;

use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::document::documents_batch::document_transition::token_issuance_transition_action::{
    TokenIssuanceTransitionAction, TokenIssuanceTransitionActionV0,
};
use dpp::state_transition::documents_batch_transition::token_issuance_transition::TokenIssuanceTransition;

/// Implement methods to transform a `TokenIssuanceTransition` into a `TokenIssuanceTransitionAction`.
impl TokenIssuanceTransitionAction {
    /// Transform a `TokenIssuanceTransition` into a `TokenIssuanceTransitionAction` using the provided data contract lookup.
    ///
    /// # Arguments
    ///
    /// * `value` - A `TokenIssuanceTransition` instance.
    /// * `get_data_contract` - A closure that fetches the DataContractFetchInfo given a contract ID.
    ///
    /// # Returns
    ///
    /// * `Result<TokenIssuanceTransitionAction, ProtocolError>` - A `TokenIssuanceTransitionAction` if successful, otherwise `ProtocolError`.
    pub fn from_token_issuance_transition_with_contract_lookup(
        value: TokenIssuanceTransition,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        match value {
            TokenIssuanceTransition::V0(v0) => {
                let v0_action =
                    TokenIssuanceTransitionActionV0::try_from_token_issuance_transition_with_contract_lookup(
                        v0,
                        get_data_contract,
                    )?;
                Ok(v0_action.into())
            }
        }
    }

    /// Transform a borrowed `TokenIssuanceTransition` into a `TokenIssuanceTransitionAction` using the provided data contract lookup.
    ///
    /// # Arguments
    ///
    /// * `value` - A reference to a `TokenIssuanceTransition`.
    /// * `get_data_contract` - A closure that fetches the DataContractFetchInfo given a contract ID.
    ///
    /// # Returns
    ///
    /// * `Result<TokenIssuanceTransitionAction, ProtocolError>` - A `TokenIssuanceTransitionAction` if successful, otherwise `ProtocolError`.
    pub fn from_borrowed_token_issuance_transition_with_contract_lookup(
        value: &TokenIssuanceTransition,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        match value {
            TokenIssuanceTransition::V0(v0) => {
                let v0_action = TokenIssuanceTransitionActionV0::try_from_borrowed_token_issuance_transition_with_contract_lookup(
                    v0,
                    get_data_contract,
                )?;
                Ok(v0_action.into())
            }
        }
    }
}