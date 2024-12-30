use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::group::GroupStateTransitionInfo;
use dpp::platform_value::Identifier;
use grovedb::TransactionArg;
use std::sync::Arc;

use dpp::ProtocolError;
use dpp::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
use crate::drive::contract::DataContractFetchInfo;
use crate::drive::Drive;
use crate::state_transition_action::document::documents_batch::document_transition::token_base_transition_action::TokenBaseTransitionActionV0;

impl TokenBaseTransitionActionV0 {
    /// try from base transition with contract lookup
    pub fn try_from_base_transition_with_contract_lookup(
        drive: &Drive,
        transaction: TransactionArg,
        value: TokenBaseTransitionV0,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        let TokenBaseTransitionV0 {
            token_contract_position,
            data_contract_id,
            identity_contract_nonce,
            token_id,
            using_group,
        } = value;

        let data_contract = get_data_contract(data_contract_id)?;

        let (store_in_group, perform_action) = match using_group {
            None => (None, true),
            Some(GroupStateTransitionInfo {
                group_contract_position,
                action_id,
            }) => {
                drive.fetch_action_id_signers(data_contract_id, group_contract_position, action_id)
            }
        };
        Ok(TokenBaseTransitionActionV0 {
            token_id,
            identity_contract_nonce,
            token_contract_position,
            data_contract,
            store_in_group,
            perform_action,
        })
    }

    /// try from borrowed base transition with contract lookup
    pub fn try_from_borrowed_base_transition_with_contract_lookup(
        drive: &Drive,
        transaction: TransactionArg,
        value: &TokenBaseTransitionV0,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
    ) -> Result<Self, ProtocolError> {
        let TokenBaseTransitionV0 {
            token_contract_position,
            data_contract_id,
            identity_contract_nonce,
            token_id,
            using_group,
        } = value;
        Ok(TokenBaseTransitionActionV0 {
            token_id: *token_id,
            identity_contract_nonce: *identity_contract_nonce,
            token_contract_position: *token_contract_position,
            data_contract: get_data_contract(*data_contract_id)?,
            store_in_group: None,
            perform_action: false,
        })
    }
}
