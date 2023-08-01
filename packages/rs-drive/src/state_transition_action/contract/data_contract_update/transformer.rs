use crate::state_transition_action::contract::data_contract_update::v0::DataContractUpdateTransitionActionV0;
use crate::state_transition_action::contract::data_contract_update::DataContractUpdateTransitionAction;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::ProtocolError;
use platform_version::version::PlatformVersion;
use platform_version::TryFromPlatformVersioned;

impl TryFromPlatformVersioned<DataContractUpdateTransition> for DataContractUpdateTransitionAction {
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: DataContractUpdateTransition,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        match value {
            DataContractUpdateTransition::V0(v0) => Ok(
                DataContractUpdateTransitionActionV0::try_from_platform_versioned(
                    v0,
                    platform_version,
                )?
                .into(),
            ),
        }
    }
}

impl TryFromPlatformVersioned<&DataContractUpdateTransition>
    for DataContractUpdateTransitionAction
{
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: &DataContractUpdateTransition,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        match value {
            DataContractUpdateTransition::V0(v0) => Ok(
                DataContractUpdateTransitionActionV0::try_from_platform_versioned(
                    v0,
                    platform_version,
                )?
                .into(),
            ),
        }
    }
}
