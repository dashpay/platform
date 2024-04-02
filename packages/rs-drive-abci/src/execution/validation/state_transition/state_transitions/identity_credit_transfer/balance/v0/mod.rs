use crate::error::Error;
use dpp::consensus::state::identity::IdentityInsufficientBalanceError;
use dpp::identity::PartialIdentity;
use dpp::state_transition::identity_credit_transfer_transition::accessors::IdentityCreditTransferTransitionAccessorsV0;
use dpp::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;

use dpp::validation::SimpleConsensusValidationResult;

use crate::error::execution::ExecutionError;
use dpp::version::PlatformVersion;

pub(in crate::execution::validation::state_transition::state_transitions) trait IdentityCreditTransferTransitionBalanceValidationV0
{
    fn validate_advanced_minimum_balance_pre_check_v0(
        &self,
        identity: &PartialIdentity,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl IdentityCreditTransferTransitionBalanceValidationV0 for IdentityCreditTransferTransition {
    fn validate_advanced_minimum_balance_pre_check_v0(
        &self,
        identity: &PartialIdentity,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let balance =
            identity
                .balance
                .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "expected to have a balance on identity for credit transfer transition",
                )))?;

        if balance < self.amount().checked_add(platform_version.fee_version.state_transition_min_fees.credit_transfer).ok_or(Error::Execution(ExecutionError::Overflow("overflow when adding amount and min_leftover_credits_before_processing in identity credit transfer")))? {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                IdentityInsufficientBalanceError::new(self.identity_id(), balance, self.amount())
                    .into(),
            ));
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
