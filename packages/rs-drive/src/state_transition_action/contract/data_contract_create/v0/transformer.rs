use crate::state_transition_action::contract::data_contract_create::v0::DataContractCreateTransitionActionV0;
use dpp::prelude::DataContract;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;
use dpp::validation::operations::ProtocolValidationOperation;
use dpp::ProtocolError;
use platform_version::version::PlatformVersion;

impl DataContractCreateTransitionActionV0 {
    pub(in crate::state_transition_action::contract::data_contract_create) fn try_from_transition(
        value: DataContractCreateTransitionV0,
        full_validation: bool,
        validation_operations: &mut Vec<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        Ok(DataContractCreateTransitionActionV0 {
            data_contract: DataContract::try_from_platform_versioned(
                value.data_contract,
                full_validation,
                validation_operations,
                platform_version,
            )?,
            identity_nonce: value.identity_nonce,
            user_fee_increase: value.user_fee_increase,
        })
    }

    pub(in crate::state_transition_action::contract::data_contract_create) fn try_from_borrowed_transition(
        value: &DataContractCreateTransitionV0,
        full_validation: bool,
        validation_operations: &mut Vec<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        Ok(DataContractCreateTransitionActionV0 {
            data_contract: DataContract::try_from_platform_versioned(
                value.data_contract.clone(),
                full_validation,
                validation_operations,
                platform_version,
            )?,
            identity_nonce: value.identity_nonce,
            user_fee_increase: value.user_fee_increase,
        })
    }
}
