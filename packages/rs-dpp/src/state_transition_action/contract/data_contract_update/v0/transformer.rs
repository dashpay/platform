use crate::state_transition::data_contract_update_transition::DataContractUpdateTransitionV0;
use crate::state_transition_action::contract::data_contract_update::v0::DataContractUpdateTransitionActionV0;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;
use platform_version::{TryFromPlatformVersioned, TryIntoPlatformVersioned};

impl TryFromPlatformVersioned<DataContractUpdateTransitionV0>
    for DataContractUpdateTransitionActionV0
{
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: DataContractUpdateTransitionV0,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        Ok(DataContractUpdateTransitionActionV0 {
            data_contract: value
                .data_contract
                .try_into_platform_versioned(platform_version)?,
        })
    }
}

impl TryFromPlatformVersioned<&DataContractUpdateTransitionV0>
    for DataContractUpdateTransitionActionV0
{
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: &DataContractUpdateTransitionV0,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        Ok(DataContractUpdateTransitionActionV0 {
            data_contract: value
                .clone()
                .data_contract
                .try_into_platform_versioned(platform_version)?,
        })
    }
}
