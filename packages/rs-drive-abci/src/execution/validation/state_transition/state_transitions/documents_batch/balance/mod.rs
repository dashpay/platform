use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::validation::state_transition::documents_batch::balance::v0::DocumentsBatchTransitionBalanceValidationV0;
use crate::execution::validation::state_transition::processor::v0::StateTransitionBalanceValidationV0;
use dpp::identity::PartialIdentity;
use dpp::state_transition::documents_batch_transition::DocumentsBatchTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

pub(crate) mod v0;
impl StateTransitionBalanceValidationV0 for DocumentsBatchTransition {
    fn validate_minimum_balance_pre_check(
        &self,
        identity: &PartialIdentity,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .documents_batch_state_transition
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
