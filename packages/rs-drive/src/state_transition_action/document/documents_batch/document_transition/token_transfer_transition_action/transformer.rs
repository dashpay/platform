use std::sync::Arc;

use dpp::platform_value::Identifier;
use dpp::ProtocolError;
use dpp::state_transition::documents_batch_transition::TokenTransferTransition;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::document::documents_batch::document_transition::token_transfer_transition_action::TokenTransferTransitionAction;
use crate::state_transition_action::document::documents_batch::document_transition::token_transfer_transition_action::v0::TokenTransferTransitionActionV0;

/// Implement methods to transform a `TokenTransferTransition` into a `TokenTransferTransitionAction`.
impl TokenTransferTransitionAction {
    /// Transform a `TokenTransferTransition` into a `TokenTransferTransitionAction` using the provided data contract lookup.
    ///
    /// # Arguments
    ///
    /// * `value` - A `TokenTransferTransition` instance.
    /// * `get_data_contract` - A closure that fetches the DataContractFetchInfo given a contract ID.
    ///
    /// # Returns
    ///
    /// * `Result<TokenTransferTransitionAction, ProtocolError>` - A `TokenTransferTransitionAction` if successful, otherwise `ProtocolError`.
    pub fn from_token_transfer_transition_with_contract_lookup(
        value: TokenTransferTransition,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        match value {
            TokenTransferTransition::V0(v0) => {
                let v0_action =
                    TokenTransferTransitionActionV0::try_from_token_transfer_transition_with_contract_lookup(
                        v0,
                        get_data_contract,
                    )?;
                Ok(v0_action.into())
            }
        }
    }

    /// Transform a borrowed `TokenTransferTransition` into a `TokenTransferTransitionAction` using the provided data contract lookup.
    ///
    /// # Arguments
    ///
    /// * `value` - A reference to a `TokenTransferTransition`.
    /// * `get_data_contract` - A closure that fetches the DataContractFetchInfo given a contract ID.
    ///
    /// # Returns
    ///
    /// * `Result<TokenTransferTransitionAction, ProtocolError>` - A `TokenTransferTransitionAction` if successful, otherwise `ProtocolError`.
    pub fn from_borrowed_token_transfer_transition_with_contract_lookup(
        value: &TokenTransferTransition,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        match value {
            TokenTransferTransition::V0(v0) => {
                let v0_action = TokenTransferTransitionActionV0::try_from_borrowed_token_transfer_transition_with_contract_lookup(
                    v0,
                    get_data_contract,
                )?;
                Ok(v0_action.into())
            }
        }
    }
}
