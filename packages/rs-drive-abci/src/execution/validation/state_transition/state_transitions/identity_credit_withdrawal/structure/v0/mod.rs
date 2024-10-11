use crate::error::Error;
use dpp::errors::consensus::basic::{BasicError, UnsupportedFeatureError};
use dpp::errors::consensus::ConsensusError;
use dpp::state_transition::state_transitions::identity::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

pub(in crate::execution::validation::state_transition::state_transitions::identity_credit_withdrawal) trait IdentityCreditWithdrawalStateTransitionStructureValidationV0 {
    fn validate_basic_structure_v0(&self, platform_version: &PlatformVersion) -> Result<SimpleConsensusValidationResult, Error>;
}

impl IdentityCreditWithdrawalStateTransitionStructureValidationV0
    for IdentityCreditWithdrawalTransition
{
    fn validate_basic_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        // This is basically disabled, return that it is not enabled

        let error = SimpleConsensusValidationResult::new_with_error(ConsensusError::BasicError(
            BasicError::UnsupportedFeatureError(UnsupportedFeatureError::new(
                "identity credit withdrawals".to_string(),
                platform_version.protocol_version,
            )),
        ));

        Ok(error)
    }
}
