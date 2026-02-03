use crate::drive::Drive;
use crate::error::Error;
use crate::state_transition_action::contract::data_contract_update::v0::DataContractUpdateTransitionActionV0;
use crate::state_transition_action::contract::data_contract_update::v1::DataContractUpdateTransitionActionV1;
use crate::state_transition_action::contract::data_contract_update::DataContractUpdateTransitionAction;
use dpp::block::block_info::BlockInfo;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::validation::operations::ProtocolValidationOperation;
use dpp::validation::ConsensusValidationResult;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl DataContractUpdateTransitionAction {
    /// Tries to transform the borrowed DataContractUpdateTransition into a DataContractUpdateTransitionAction.
    /// This method supports both V0 and V1 transitions.
    /// V1 transitions require Drive access to fetch the old contract from state.
    ///
    /// If validation is true, the data contract transformation verifies that the data contract is valid.
    /// If validation is false, the data contract base structure is created regardless of if it is valid.
    pub fn try_from_borrowed_transition(
        value: &DataContractUpdateTransition,
        drive: &Drive,
        transaction: TransactionArg,
        block_info: &BlockInfo,
        full_validation: bool,
        validation_operations: &mut Vec<ProtocolValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<Self>, Error> {
        match value {
            DataContractUpdateTransition::V0(v0) => Ok(ConsensusValidationResult::new_with_data(
                DataContractUpdateTransitionActionV0::try_from_borrowed_transition(
                    v0,
                    block_info,
                    full_validation,
                    validation_operations,
                    platform_version,
                )
                .map_err(|e| Error::Protocol(Box::new(e)))?
                .into(),
            )),
            DataContractUpdateTransition::V1(v1) => {
                let validation_result = DataContractUpdateTransitionActionV1::try_from_transition(
                    v1,
                    drive,
                    transaction,
                    block_info,
                    full_validation,
                    validation_operations,
                    platform_version,
                )?;
                Ok(validation_result.map(|action| action.into()))
            }
        }
    }
}
