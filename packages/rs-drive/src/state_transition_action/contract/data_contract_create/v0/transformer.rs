use crate::state_transition_action::contract::data_contract_create::v0::DataContractCreateTransitionActionV0;
use dpp::prelude::DataContract;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;
use dpp::ProtocolError;
use platform_version::version::PlatformVersion;
use platform_version::TryFromPlatformVersioned;

impl TryFromPlatformVersioned<DataContractCreateTransitionV0>
    for DataContractCreateTransitionActionV0
{
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: DataContractCreateTransitionV0,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        Ok(DataContractCreateTransitionActionV0 {
            data_contract: DataContract::try_from_platform_versioned(
                value.data_contract,
                true,
                platform_version,
            )?,
            identity_nonce: value.identity_nonce,
            user_fee_increase: value.user_fee_increase,
        })
    }
}

impl TryFromPlatformVersioned<&DataContractCreateTransitionV0>
    for DataContractCreateTransitionActionV0
{
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: &DataContractCreateTransitionV0,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        Ok(DataContractCreateTransitionActionV0 {
            data_contract: DataContract::try_from_platform_versioned(
                value.data_contract.clone(),
                true,
                platform_version,
            )?,
            identity_nonce: value.identity_nonce,
            user_fee_increase: value.user_fee_increase,
        })
    }
}
