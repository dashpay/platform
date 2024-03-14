use crate::state_transition_action::contract::data_contract_create::v0::DataContractCreateTransitionActionV0;
use crate::state_transition_action::contract::data_contract_create::DataContractCreateTransitionAction;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::ProtocolError;
use platform_version::version::PlatformVersion;

impl DataContractCreateTransitionAction {
    /// tries to transform the DataContractCreateTransition into a DataContractCreateTransitionAction
    /// if validation is true the data contract transformation verifies that the data contract is valid
    /// if validation is false, the data contract base structure is created regardless of if it is valid
    pub fn try_from_transition(
        value: DataContractCreateTransition,
        validate: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match value {
            DataContractCreateTransition::V0(v0) => {
                Ok(DataContractCreateTransitionActionV0::try_from_transition(
                    v0,
                    validate,
                    platform_version,
                )?
                .into())
            }
        }
    }

    /// tries to transform the borrowed DataContractCreateTransition into a DataContractCreateTransitionAction
    /// if validation is true the data contract transformation verifies that the data contract is valid
    /// if validation is false, the data contract base structure is created regardless of if it is valid

    pub fn try_from_borrowed_transition(
        value: &DataContractCreateTransition,
        validate: bool,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match value {
            DataContractCreateTransition::V0(v0) => Ok(
                DataContractCreateTransitionActionV0::try_from_borrowed_transition(
                    v0,
                    validate,
                    platform_version,
                )?
                .into(),
            ),
        }
    }
}
