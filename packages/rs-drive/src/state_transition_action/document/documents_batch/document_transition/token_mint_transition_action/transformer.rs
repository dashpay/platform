use dpp::platform_value::Identifier;
use dpp::ProtocolError;
use grovedb::TransactionArg;
use std::sync::Arc;

use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::document::documents_batch::document_transition::token_mint_transition_action::{TokenMintTransitionActionV0, TokenMintTransitionAction};
use dpp::state_transition::batch_transition::token_issuance_transition::TokenMintTransition;
use crate::drive::Drive;

/// Implement methods to transform a `TokenMintTransition` into a `TokenMintTransitionAction`.
impl TokenMintTransitionAction {
    /// Transform a `TokenMintTransition` into a `TokenMintTransitionAction` using the provided data contract lookup.
    ///
    /// # Arguments
    ///
    /// * `value` - A `TokenMintTransition` instance.
    /// * `get_data_contract` - A closure that fetches the DataContractFetchInfo given a contract ID.
    ///
    /// # Returns
    ///
    /// * `Result<TokenMintTransitionAction, ProtocolError>` - A `TokenMintTransitionAction` if successful, otherwise `ProtocolError`.
    pub fn from_token_mint_transition_with_contract_lookup(
        drive: &Drive,
        transaction: TransactionArg,
        value: TokenMintTransition,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        match value {
            TokenMintTransition::V0(v0) => {
                let v0_action =
                    TokenMintTransitionActionV0::try_from_token_mint_transition_with_contract_lookup(
                        drive,
                        transaction,
                        v0,
                        get_data_contract,
                    )?;
                Ok(v0_action.into())
            }
        }
    }

    /// Transform a borrowed `TokenMintTransition` into a `TokenMintTransitionAction` using the provided data contract lookup.
    ///
    /// # Arguments
    ///
    /// * `value` - A reference to a `TokenMintTransition`.
    /// * `get_data_contract` - A closure that fetches the DataContractFetchInfo given a contract ID.
    ///
    /// # Returns
    ///
    /// * `Result<TokenMintTransitionAction, ProtocolError>` - A `TokenMintTransitionAction` if successful, otherwise `ProtocolError`.
    pub fn try_from_borrowed_token_mint_transition_with_contract_lookup(
        drive: &Drive,
        transaction: TransactionArg,
        value: &TokenMintTransition,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        match value {
            TokenMintTransition::V0(v0) => {
                let v0_action = TokenMintTransitionActionV0::try_from_borrowed_token_mint_transition_with_contract_lookup(
                    drive,
                    transaction,
                    v0,
                    get_data_contract,
                )?;
                Ok(v0_action.into())
            }
        }
    }
}
