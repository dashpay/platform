use std::collections::HashMap;
use dpp::group::{GroupStateTransitionInfo, GroupStateTransitionResolvedInfo};
use dpp::platform_value::Identifier;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::sync::Arc;
use grovedb::batch::KeyInfoPath;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::group::accessors::v0::GroupV0Getters;
use dpp::ProtocolError;
use dpp::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
use platform_version::version::PlatformVersion;
use crate::drive::contract::DataContractFetchInfo;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::state_transition_action::document::documents_batch::document_transition::token_base_transition_action::TokenBaseTransitionActionV0;

impl TokenBaseTransitionActionV0 {
    /// try from base transition with contract lookup
    pub fn try_from_base_transition_with_contract_lookup(
        drive: &Drive,
        owner_id: Identifier,
        value: TokenBaseTransitionV0,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        let TokenBaseTransitionV0 {
            token_contract_position,
            data_contract_id,
            identity_contract_nonce,
            token_id,
            using_group_info,
        } = value;

        let data_contract = get_data_contract(data_contract_id)?;

        let (perform_action, store_in_group) = match using_group_info {
            None => (true, None),
            Some(GroupStateTransitionInfo {
                group_contract_position,
                action_id,
                action_is_proposer,
            }) => {
                let group = data_contract.contract.group(*group_contract_position)?;
                let signer_power = group.member_power(owner_id)?;
                let required_power = group.required_power();
                let current_power = drive.fetch_action_id_signers_power_and_add_operations(
                    data_contract_id,
                    group_contract_position,
                    action_id,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    drive_operations,
                    platform_version,
                )?;
                let perform_action = current_power + signer_power >= required_power;
                let store_in_group = GroupStateTransitionResolvedInfo {
                    group_contract_position,
                    group: group.clone(),
                    action_id,
                    action_is_proposer,
                    signer_power,
                };
                (perform_action, Some(store_in_group))
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
        owner_id: Identifier,
        value: &TokenBaseTransitionV0,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        get_data_contract: impl Fn(Identifier) -> Result<Arc<DataContractFetchInfo>, ProtocolError>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        let TokenBaseTransitionV0 {
            token_contract_position,
            data_contract_id,
            identity_contract_nonce,
            token_id,
            using_group_info,
        } = value;

        let data_contract = get_data_contract(*data_contract_id)?;

        let (perform_action, store_in_group) = match using_group_info {
            None => (true, None),
            Some(GroupStateTransitionInfo {
                group_contract_position,
                action_id,
                action_is_proposer,
            }) => {
                let group = data_contract.contract.group(*group_contract_position)?;
                let signer_power = group.member_power(owner_id)?;
                let required_power = group.required_power();
                let current_power = drive.fetch_action_id_signers_power_and_add_operations(
                    *data_contract_id,
                    *group_contract_position,
                    *action_id,
                    estimated_costs_only_with_layer_info,
                    transaction,
                    drive_operations,
                    platform_version,
                )?;
                let perform_action = current_power + signer_power >= required_power;
                let store_in_group = GroupStateTransitionResolvedInfo {
                    group_contract_position: *group_contract_position,
                    group: group.clone(),
                    action_id: *action_id,
                    action_is_proposer: *action_is_proposer,
                    signer_power,
                };
                (perform_action, Some(store_in_group))
            }
        };
        Ok(TokenBaseTransitionActionV0 {
            token_id: *token_id,
            identity_contract_nonce: *identity_contract_nonce,
            token_contract_position: *token_contract_position,
            data_contract,
            store_in_group,
            perform_action,
        })
    }
}
