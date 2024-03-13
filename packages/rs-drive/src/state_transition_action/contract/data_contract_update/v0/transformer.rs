use crate::state_transition_action::contract::data_contract_update::v0::DataContractUpdateTransitionActionV0;
use dpp::data_contract::DataContract;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransitionV0;
use dpp::ProtocolError;
use platform_version::version::PlatformVersion;

impl DataContractUpdateTransitionActionV0 {
    pub(in crate::state_transition_action::contract::data_contract_update) fn try_from_transition(
        value: DataContractUpdateTransitionV0,
        validate: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        Ok(DataContractUpdateTransitionActionV0 {
            data_contract: DataContract::try_from_platform_versioned(
                value.data_contract,
                validate,
                platform_version,
            )?,
            identity_contract_nonce: value.identity_contract_nonce,
            user_fee_increase: value.user_fee_increase,
        })
    }

    pub(in crate::state_transition_action::contract::data_contract_update) fn try_from_borrowed_transition(
        value: &DataContractUpdateTransitionV0,
        validate: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        Ok(DataContractUpdateTransitionActionV0 {
            data_contract: DataContract::try_from_platform_versioned(
                value.data_contract.clone(),
                validate,
                platform_version,
            )?,
            identity_contract_nonce: value.identity_contract_nonce,
            user_fee_increase: value.user_fee_increase,
        })
    }
}
