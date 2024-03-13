use crate::state_transition_action::contract::data_contract_update::v0::DataContractUpdateTransitionActionV0;
use crate::state_transition_action::contract::data_contract_update::DataContractUpdateTransitionAction;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::ProtocolError;
use platform_version::version::PlatformVersion;

impl DataContractUpdateTransitionAction {
    /// tries to transform the DataContractUpdateTransition into a DataContractUpdateTransitionAction
    /// if validation is true the data contract transformation verifies that the data contract is valid
    /// if validation is false, the data contract base structure is created regardless of if it is valid
    pub fn try_from_transition(
        value: DataContractUpdateTransition,
        validate: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match value {
            DataContractUpdateTransition::V0(v0) => {
                Ok(DataContractUpdateTransitionActionV0::try_from_transition(
                    v0,
                    validate,
                    platform_version,
                )?
                .into())
            }
        }
    }

    /// tries to transform the borrowed DataContractUpdateTransition into a DataContractUpdateTransitionAction
    /// if validation is true the data contract transformation verifies that the data contract is valid
    /// if validation is false, the data contract base structure is created regardless of if it is valid

    pub fn try_from_borrowed_transition(
        value: &DataContractUpdateTransition,
        validate: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match value {
            DataContractUpdateTransition::V0(v0) => Ok(
                DataContractUpdateTransitionActionV0::try_from_borrowed_transition(
                    v0,
                    validate,
                    platform_version,
                )?
                .into(),
            ),
        }
    }
}
