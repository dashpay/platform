use crate::state_transition_action::contract::data_contract_create::v0::DataContractCreateTransitionActionV0;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;
use dpp::ProtocolError;
use platform_version::version::PlatformVersion;
use platform_version::{TryFromPlatformVersioned, TryIntoPlatformVersioned};
use serde::{Deserialize, Serialize};
use std::convert::TryInto;

impl TryFromPlatformVersioned<DataContractCreateTransitionV0>
    for DataContractCreateTransitionActionV0
{
    type Error = ProtocolError;

    fn try_from_platform_versioned(
        value: DataContractCreateTransitionV0,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Self::Error> {
        Ok(DataContractCreateTransitionActionV0 {
            data_contract: value
                .data_contract
                .try_into_platform_versioned(platform_version)?,
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
            data_contract: value
                .clone()
                .data_contract
                .try_into_platform_versioned(platform_version)?,
        })
    }
}
