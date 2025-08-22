use crate::state_transition_action::contract::data_contract_create::v0::DataContractCreateTransitionActionV0;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v1::DataContractV1Setters;
use dpp::prelude::DataContract;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;
use dpp::validation::operations::ProtocolValidationOperation;
use dpp::ProtocolError;
use platform_version::version::PlatformVersion;

impl DataContractCreateTransitionActionV0 {
    pub(in crate::state_transition_action::contract::data_contract_create) fn try_from_transition(
        value: DataContractCreateTransitionV0,
        block_info: &BlockInfo,
        full_validation: bool,
        validation_operations: &mut Vec<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let mut data_contract = DataContract::try_from_platform_versioned(
            value.data_contract,
            full_validation,
            validation_operations,
            platform_version,
        )?;
        data_contract.set_created_at(Some(block_info.time_ms));
        data_contract.set_created_at_epoch(Some(block_info.epoch.index));
        data_contract.set_created_at_block_height(Some(block_info.height));
        Ok(DataContractCreateTransitionActionV0 {
            data_contract,
            identity_nonce: value.identity_nonce,
            user_fee_increase: value.user_fee_increase,
        })
    }

    pub(in crate::state_transition_action::contract::data_contract_create) fn try_from_borrowed_transition(
        value: &DataContractCreateTransitionV0,
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
        Ok(DataContractCreateTransitionActionV0 {
            data_contract,
            identity_nonce: value.identity_nonce,
            user_fee_increase: value.user_fee_increase,
        })
    }
}
