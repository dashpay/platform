use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::validation::state_transition::processor::v0::StateTransitionAllowanceValidationV0;
use crate::platform_types::platform::PlatformRef;
use dpp::state_transition::documents_batch_transition::DocumentsBatchTransition;
use dpp::validation::ConsensusValidationResult;
use dpp::version::PlatformVersion;

mod v0;

impl StateTransitionAllowanceValidationV0 for DocumentsBatchTransition {
    fn has_allowance_validation(&self, platform_version: &PlatformVersion) -> Result<bool, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .documents_batch_state_transition
            .allowance
        {
            0 => Ok(true),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "StateTransition::validate_temporary_disabled_contested_documents"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Disable contested document create transitions for the first 3 epochs
    fn validate_allowance<C>(
        &self,
        platform: &PlatformRef<C>,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<()>, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .documents_batch_state_transition
            .allowance
        {
            0 => Ok(v0::validate_allowance_v0(self, platform)),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "StateTransition::validate_temporary_disabled_contested_documents"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
