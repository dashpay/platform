use crate::state_transition_action::contract::data_contract_create::v0::DataContractCreateTransitionActionV0;
use crate::state_transition_action::contract::data_contract_create::DataContractCreateTransitionAction;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::ProtocolError;
use platform_version::version::PlatformVersion;
use platform_version::TryFromPlatformVersioned;

impl TryFromPlatformVersioned<DataContractCreateTransition> for DataContractCreateTransitionAction {
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: DataContractCreateTransition,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        match value {
            DataContractCreateTransition::V0(v0) => Ok(
                DataContractCreateTransitionActionV0::try_from_platform_versioned(
                    v0,
                    platform_version,
                )?
                .into(),
            ),
        }
    }
}

impl TryFromPlatformVersioned<&DataContractCreateTransition>
    for DataContractCreateTransitionAction
{
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: &DataContractCreateTransition,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        match value {
            DataContractCreateTransition::V0(v0) => Ok(
                DataContractCreateTransitionActionV0::try_from_platform_versioned(
                    v0,
                    platform_version,
                )?
                .into(),
            ),
        }
    }
}
