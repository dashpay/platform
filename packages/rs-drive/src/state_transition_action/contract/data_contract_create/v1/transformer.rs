use crate::state_transition_action::contract::data_contract_create::v1::DataContractCreateTransitionActionV1;
use dpp::block::block_info::BlockInfo;
use dpp::contract_group::{generate_contract_group_id, ContractGroupInfo};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v1::DataContractV1Setters;
use dpp::prelude::DataContract;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransitionV1;
use dpp::validation::operations::ProtocolValidationOperation;
use dpp::ProtocolError;
use platform_version::version::PlatformVersion;

impl DataContractCreateTransitionActionV1 {
    pub(in crate::state_transition_action::contract::data_contract_create) fn try_from_transition(
        value: DataContractCreateTransitionV1,
        block_info: &BlockInfo,
        full_validation: bool,
        validation_operations: &mut Vec<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let DataContractCreateTransitionV1 {
            data_contract,
            identity_nonce,
            contract_group,
            contract_group_memberships,
            user_fee_increase,
            ..
        } = value;
        let mut data_contract = DataContract::try_from_platform_versioned(
            data_contract,
            full_validation,
            validation_operations,
            platform_version,
        )?;
        data_contract.set_created_at(Some(block_info.time_ms));
        data_contract.set_created_at_epoch(Some(block_info.epoch.index));
        data_contract.set_created_at_block_height(Some(block_info.height));
        let contract_group = contract_group.map(|registration| {
            (
                generate_contract_group_id(&data_contract.owner_id(), identity_nonce),
                ContractGroupInfo::from(registration),
            )
        });
        Ok(DataContractCreateTransitionActionV1 {
            data_contract,
            identity_nonce,
            user_fee_increase,
            contract_group,
            contract_group_memberships,
        })
    }

    pub(in crate::state_transition_action::contract::data_contract_create) fn try_from_borrowed_transition(
        value: &DataContractCreateTransitionV1,
        block_info: &BlockInfo,
        full_validation: bool,
        validation_operations: &mut Vec<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let mut data_contract = DataContract::try_from_platform_versioned(
            value.data_contract.clone(),
            full_validation,
            validation_operations,
            platform_version,
        )?;
        data_contract.set_created_at(Some(block_info.time_ms));
        data_contract.set_created_at_epoch(Some(block_info.epoch.index));
        data_contract.set_created_at_block_height(Some(block_info.height));
        let contract_group = value.contract_group.as_ref().map(|registration| {
            (
                generate_contract_group_id(&data_contract.owner_id(), value.identity_nonce),
                ContractGroupInfo::from(registration),
            )
        });
        Ok(DataContractCreateTransitionActionV1 {
            data_contract,
            identity_nonce: value.identity_nonce,
            user_fee_increase: value.user_fee_increase,
            contract_group,
            contract_group_memberships: value.contract_group_memberships.clone(),
        })
    }
}
