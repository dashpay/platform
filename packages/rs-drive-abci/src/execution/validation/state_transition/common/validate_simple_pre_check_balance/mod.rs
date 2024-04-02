use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::validation::state_transition::common::validate_simple_pre_check_balance::v0::ValidateSimplePreCheckBalanceV0;
use dpp::identity::PartialIdentity;
use dpp::state_transition::StateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
pub mod v0;

pub trait ValidateSimplePreCheckBalance {
    fn validate_simple_pre_check_minimum_balance(
        &self,
        identity: &PartialIdentity,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl ValidateSimplePreCheckBalance for StateTransition {
    fn validate_simple_pre_check_minimum_balance(
        &self,
        identity: &PartialIdentity,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .common_validation_methods
            .validate_simple_pre_check_balance
        {
            0 => self.validate_simple_pre_check_minimum_balance_v0(identity, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "StateTransition::validate_simple_pre_check_balance".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
