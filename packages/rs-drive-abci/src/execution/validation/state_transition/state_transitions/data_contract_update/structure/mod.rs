use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::validation::state_transition::data_contract_update::structure::v0::DataContractUpdateStateTransitionStructureValidationV0;
use crate::execution::validation::state_transition::processor::v0::StateTransitionBasicStructureValidationV0;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

pub(crate) mod v0;

impl StateTransitionBasicStructureValidationV0 for DataContractUpdateTransition {
    fn validate_basic_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .contract_update_state_transition
            .base_structure
        {
            0 => self.validate_base_structure_v0(platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "data contract update transition: validate_basic_structure".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
