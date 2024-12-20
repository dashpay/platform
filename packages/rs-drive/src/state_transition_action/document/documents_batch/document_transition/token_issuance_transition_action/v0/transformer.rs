use std::sync::Arc;

use dpp::identifier::Identifier;
use dpp::state_transition::documents_batch_transition::token_issuance_transition::v0::TokenIssuanceTransitionV0;
use dpp::ProtocolError;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::state_transition::documents_batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use dpp::tokens::errors::TokenError;
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::document::documents_batch::document_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0};
use crate::state_transition_action::document::documents_batch::document_transition::token_issuance_transition_action::v0::TokenIssuanceTransitionActionV0;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;

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
        let TokenIssuanceTransitionV0 {
            base,
            issued_to_identity_id,
            amount,
        } = value;

        let position = base.token_contract_position();

        let base_action = TokenBaseTransitionAction::try_from_base_transition_with_contract_lookup(
            base,
            get_data_contract,
        )?;

        let identity_balance_holder_id = issued_to_identity_id
            .or_else(|| {
                base_action
                    .data_contract_fetch_info_ref()
                    .contract
                    .tokens()
                    .get(&position)
                    .and_then(|token_configuration| {
                        token_configuration.new_tokens_destination_identity()
                    })
            })
            .ok_or(ProtocolError::Token(
                TokenError::DestinationIdentityForMintingNotSetError.into(),
            ))?;

        Ok(TokenIssuanceTransitionActionV0 {
            base: base_action,
            issuance_amount: amount,
            identity_balance_holder_id,
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
        let TokenIssuanceTransitionV0 {
            base,
            issued_to_identity_id,
            amount,
        } = value;

        let base_action =
            TokenBaseTransitionAction::try_from_borrowed_base_transition_with_contract_lookup(
                base,
                get_data_contract,
            )?;

        let identity_balance_holder_id = issued_to_identity_id
            .or_else(|| {
                base_action
                    .data_contract_fetch_info_ref()
                    .contract
                    .tokens()
                    .get(&base.token_contract_position())
                    .and_then(|token_configuration| {
                        token_configuration.new_tokens_destination_identity()
                    })
            })
            .ok_or(ProtocolError::Token(
                TokenError::DestinationIdentityForMintingNotSetError.into(),
            ))?;

        Ok(TokenIssuanceTransitionActionV0 {
            base: base_action,
            issuance_amount: *amount,
            identity_balance_holder_id,
        })
    }
}
