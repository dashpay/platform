use std::sync::Arc;

use dpp::platform_value::Identifier;
use dpp::ProtocolError;

use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::document::documents_batch::document_transition::token_burn_transition_action::{
    TokenBurnTransitionAction, TokenBurnTransitionActionV0,
};
use dpp::state_transition::batch_transition::token_burn_transition::TokenBurnTransition;

/// Implement methods to transform a `TokenBurnTransition` into a `TokenBurnTransitionAction`.
impl TokenBurnTransitionAction {
    /// Transform a `TokenBurnTransition` into a `TokenBurnTransitionAction` using the provided data contract lookup.
    ///
    /// # Arguments
    ///
    /// * `value` - A `TokenBurnTransition` instance.
    /// * `get_data_contract` - A closure that fetches the DataContractFetchInfo given a contract ID.
    ///
    /// # Returns
    ///
    /// * `Result<TokenBurnTransitionAction, ProtocolError>` - A `TokenBurnTransitionAction` if successful, otherwise `ProtocolError`.
    pub fn try_from_token_burn_transition_with_contract_lookup(
        value: TokenBurnTransition,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        match value {
            TokenBurnTransition::V0(v0) => {
                let v0_action =
                    TokenBurnTransitionActionV0::try_from_token_burn_transition_with_contract_lookup(
                        v0,
                        get_data_contract,
                    )?;
                Ok(v0_action.into())
            }
        }
    }

    /// Transform a borrowed `TokenBurnTransition` into a `TokenBurnTransitionAction` using the provided data contract lookup.
    ///
    /// # Arguments
    ///
    /// * `value` - A reference to a `TokenBurnTransition`.
    /// * `get_data_contract` - A closure that fetches the DataContractFetchInfo given a contract ID.
    ///
    /// # Returns
    ///
    /// * `Result<TokenBurnTransitionAction, ProtocolError>` - A `TokenBurnTransitionAction` if successful, otherwise `ProtocolError`.
    pub fn try_from_borrowed_token_burn_transition_with_contract_lookup(
        value: &TokenBurnTransition,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        match value {
            TokenBurnTransition::V0(v0) => {
                let v0_action = TokenBurnTransitionActionV0::try_from_borrowed_token_burn_transition_with_contract_lookup(
                    v0,
                    get_data_contract,
                )?;
                Ok(v0_action.into())
            }
        }
    }
}
