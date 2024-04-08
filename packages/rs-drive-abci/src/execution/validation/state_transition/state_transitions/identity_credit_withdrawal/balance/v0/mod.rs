use crate::error::Error;
use dpp::consensus::state::identity::IdentityInsufficientBalanceError;
use dpp::identity::PartialIdentity;
use dpp::state_transition::identity_credit_withdrawal_transition::accessors::IdentityCreditWithdrawalTransitionAccessorsV0;
use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;

use crate::error::execution::ExecutionError;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

pub(in crate::execution::validation::state_transition::state_transitions) trait IdentityCreditTransferTransitionBalanceValidationV0
{
    fn validate_advanced_minimum_balance_pre_check_v0(
        &self,
        identity: &PartialIdentity,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl IdentityCreditTransferTransitionBalanceValidationV0 for IdentityCreditWithdrawalTransition {
    fn validate_advanced_minimum_balance_pre_check_v0(
        &self,
        identity: &PartialIdentity,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let balance =
            identity
                .balance
                .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "expected to have a balance on identity for credit withdrawal transition",
                )))?;

        if balance < self.amount().checked_add(platform_version.fee_version.state_transition_min_fees.credit_withdrawal).ok_or(Error::Execution(ExecutionError::Overflow("overflow when adding amount and min_leftover_credits_before_processing in identity credit withdrawal")))? {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                IdentityInsufficientBalanceError::new(self.identity_id(), balance, self.amount())
                    .into(),
            ));
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
