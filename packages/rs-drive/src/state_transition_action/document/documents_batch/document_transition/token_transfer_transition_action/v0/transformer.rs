use std::sync::Arc;

use dpp::identifier::Identifier;
use dpp::state_transition::documents_batch_transition::token_transfer_transition::v0::TokenTransferTransitionV0;
use dpp::ProtocolError;

use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::document::documents_batch::document_transition::token_base_transition_action::TokenBaseTransitionAction;
use crate::state_transition_action::document::documents_batch::document_transition::token_transfer_transition_action::TokenTransferTransitionActionV0;

impl TokenTransferTransitionActionV0 {
    /// Convert a `TokenTransferTransitionV0` into a `TokenTransferTransitionActionV0` using contract lookup
    pub fn try_from_token_transfer_transition_with_contract_lookup(
        value: TokenTransferTransitionV0,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        let TokenTransferTransitionV0 {
            base,
            amount,
            recipient_owner_id,
        } = value;

        let base_action = TokenBaseTransitionAction::try_from_base_transition_with_contract_lookup(
            base,
            get_data_contract,
        )?;

        Ok(TokenTransferTransitionActionV0 {
            base: base_action,
            amount,
            recipient_id: recipient_owner_id,
        })
    }

    /// Convert a borrowed `TokenTransferTransitionV0` into a `TokenTransferTransitionActionV0` using contract lookup
    pub fn try_from_borrowed_token_transfer_transition_with_contract_lookup(
        value: &TokenTransferTransitionV0,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        let TokenTransferTransitionV0 {
            base,
            amount,
            recipient_owner_id,
        } = value;

        let base_action =
            TokenBaseTransitionAction::try_from_borrowed_base_transition_with_contract_lookup(
                &base,
                get_data_contract,
            )?;

        Ok(TokenTransferTransitionActionV0 {
            base: base_action.into(),
            amount: *amount,
            recipient_id: *recipient_owner_id,
        })
    }
}
