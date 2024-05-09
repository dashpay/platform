use crate::error::Error;
use dpp::consensus::state::identity::IdentityInsufficientBalanceError;
use dpp::identity::PartialIdentity;
use dpp::state_transition::documents_batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::documents_batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use dpp::state_transition::documents_batch_transition::DocumentsBatchTransition;

use dpp::validation::SimpleConsensusValidationResult;

use crate::error::execution::ExecutionError;
use dpp::version::PlatformVersion;

pub(in crate::execution::validation::state_transition::state_transitions) trait DocumentsBatchTransitionBalanceValidationV0
{
    fn validate_advanced_minimum_balance_pre_check_v0(
        &self,
        identity: &PartialIdentity,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DocumentsBatchTransitionBalanceValidationV0 for DocumentsBatchTransition {
    fn validate_advanced_minimum_balance_pre_check_v0(
        &self,
        identity: &PartialIdentity,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let balance =
            identity
                .balance
                .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "expected to have a balance on identity for documents batch transition",
                )))?;

        let purchases_amount = self.all_purchases_amount().unwrap_or_default();

        let base_fees = platform_version.fee_version.state_transition_min_fees.document_batch_sub_transition.checked_mul(self.transitions().len() as u64).ok_or(Error::Execution(ExecutionError::Overflow("overflow when multiplying base fee and amount of sub transitions in documents batch transition")))?;

        // This is just the needed balance to pass this validation step, most likely the actual fees are smaller
        let needed_balance = purchases_amount
            .checked_add(base_fees)
            .ok_or(Error::Execution(ExecutionError::Overflow(
            "overflow when adding all purchases amount and base fees in documents batch transition",
        )))?;

        if balance < needed_balance {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                IdentityInsufficientBalanceError::new(identity.id, balance, needed_balance).into(),
            ));
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
