use std::sync::Arc;

use dpp::identifier::Identifier;
use dpp::state_transition::documents_batch_transition::token_burn_transition::v0::TokenBurnTransitionV0;
use dpp::ProtocolError;

use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::document::documents_batch::document_transition::token_base_transition_action::TokenBaseTransitionAction;
use crate::state_transition_action::document::documents_batch::document_transition::token_burn_transition_action::v0::TokenBurnTransitionActionV0;

impl TokenBurnTransitionActionV0 {
    /// Attempt to convert a `TokenBurnTransitionV0` into a `TokenBurnTransitionActionV0` using a data contract lookup function.
    ///
    /// # Arguments
    ///
    /// * `value` - A `TokenBurnTransitionV0` from which to derive the action
    /// * `get_data_contract` - A closure that, given a `data_contract_id`, returns an `Arc<DataContractFetchInfo>`
    ///
    /// # Returns
    ///
    /// * `Result<TokenBurnTransitionActionV0, ProtocolError>` - A `TokenBurnTransitionActionV0` if successful, else `ProtocolError`.
    pub fn try_from_token_burn_transition_with_contract_lookup(
        value: TokenBurnTransitionV0,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        let TokenBurnTransitionV0 { base, burn_amount } = value;

        let base_action = TokenBaseTransitionAction::try_from_base_transition_with_contract_lookup(
            base,
            get_data_contract,
        )?;

        Ok(TokenBurnTransitionActionV0 {
            base: base_action,
            burn_amount,
        })
    }

    /// Attempt to convert a borrowed `TokenBurnTransitionV0` into a `TokenBurnTransitionActionV0` using a data contract lookup function.
    ///
    /// # Arguments
    ///
    /// * `value` - A reference to a `TokenBurnTransitionV0` from which to derive the action
    /// * `get_data_contract` - A closure that, given a `data_contract_id`, returns an `Arc<DataContractFetchInfo>`
    ///
    /// # Returns
    ///
    /// * `Result<TokenBurnTransitionActionV0, ProtocolError>` - A `TokenBurnTransitionActionV0` if successful, else `ProtocolError`.
    pub fn try_from_borrowed_token_burn_transition_with_contract_lookup(
        value: &TokenBurnTransitionV0,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        let TokenBurnTransitionV0 { base, burn_amount } = value;

        let base_action =
            TokenBaseTransitionAction::try_from_borrowed_base_transition_with_contract_lookup(
                base,
                get_data_contract,
            )?;

        Ok(TokenBurnTransitionActionV0 {
            base: base_action,
            burn_amount: *burn_amount,
        })
    }
}
