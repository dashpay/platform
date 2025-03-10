use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::validation::state_transition::batch::balance::v0::DocumentsBatchTransitionBalanceValidationV0;
use crate::execution::validation::state_transition::processor::v0::StateTransitionIdentityBalanceValidationV0;
use dpp::identity::PartialIdentity;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

pub(crate) mod v0;
impl StateTransitionIdentityBalanceValidationV0 for BatchTransition {
    fn validate_minimum_balance_pre_check(
        &self,
        identity: &PartialIdentity,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .batch_state_transition
            .balance_pre_check
        {
            0 => self.validate_advanced_minimum_balance_pre_check_v0(identity, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "documents batch transition: validate_minimum_balance_pre_check"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
