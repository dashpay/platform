use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::validation::state_transition::data_contract_update::advanced_structure::v0::DataContractUpdateStateTransitionAdvancedStructureValidationV0;
use crate::execution::validation::state_transition::processor::v0::StateTransitionAdvancedStructureValidationV0;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::validation::ConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::state_transition_action::StateTransitionAction;

pub(crate) mod v0;

impl StateTransitionAdvancedStructureValidationV0 for DataContractUpdateTransition {
    fn validate_advanced_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .contract_update_state_transition
            .advanced_structure
        {
            Some(0) => self.validate_advanced_structure_v0(platform_version),
            Some(version) => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "data contract update transition: validate_basic_structure".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Execution(ExecutionError::VersionNotActive {
                method: "data contract update transition: validate_basic_structure".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
