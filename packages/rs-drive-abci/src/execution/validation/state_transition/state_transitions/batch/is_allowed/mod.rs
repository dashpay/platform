use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::validation::state_transition::processor::StateTransitionIsAllowedValidationV0;
use crate::platform_types::platform::PlatformRef;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::validation::ConsensusValidationResult;
use dpp::version::PlatformVersion;

mod v0;

impl StateTransitionIsAllowedValidationV0 for BatchTransition {
    fn has_is_allowed_validation(&self) -> Result<bool, Error> {
        Ok(true)
    }

    /// Disable contested document create transitions for the first 3 epochs
    fn validate_is_allowed<C>(
        &self,
        platform: &PlatformRef<C>,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<()>, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .batch_state_transition
            .is_allowed
        {
            0 => Ok(v0::validate_is_allowed_v0(self, platform)),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "StateTransition::validate_is_allowed".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
