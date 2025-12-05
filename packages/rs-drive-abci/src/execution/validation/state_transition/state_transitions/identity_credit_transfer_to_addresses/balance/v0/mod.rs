use crate::error::Error;
use dpp::consensus::basic::overflow_error::OverflowError;
use dpp::consensus::state::identity::IdentityInsufficientBalanceError;
use dpp::identity::PartialIdentity;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::identity_credit_transfer_to_addresses_transition::accessors::IdentityCreditTransferToAddressesTransitionAccessorsV0;
use dpp::state_transition::identity_credit_transfer_to_addresses_transition::IdentityCreditTransferToAddressesTransition;

use dpp::validation::SimpleConsensusValidationResult;

use crate::error::execution::ExecutionError;
use dpp::version::PlatformVersion;

pub(in crate::execution::validation::state_transition::state_transitions) trait IdentityCreditTransferToAddressesTransitionBalanceValidationV0
{
    fn validate_advanced_minimum_balance_pre_check_v0(
        &self,
        identity: &PartialIdentity,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl IdentityCreditTransferToAddressesTransitionBalanceValidationV0
    for IdentityCreditTransferToAddressesTransition
{
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

        // Safely sum all recipient amounts, checking for overflow
        let amount = match self
            .recipient_addresses()
            .values()
            .try_fold(0u64, |acc, &val| acc.checked_add(val))
        {
            Some(sum) => sum,
            None => {
                return Ok(ConsensusValidationResult::new_with_error(
                    OverflowError::new("recipient addresses sum overflow".to_string()).into(),
                ));
            }
        };

        let min_fees = &platform_version.fee_version.state_transition_min_fees;
        let output_count = self.recipient_addresses().len() as u64;
        let required_fee = min_fees.credit_transfer_to_addresses.saturating_add(
            min_fees
                .address_funds_transfer_output_cost
                .saturating_mul(output_count),
        );

        if balance < amount.checked_add(required_fee).ok_or(Error::Execution(ExecutionError::Overflow("overflow when adding amount and min_leftover_credits_before_processing in identity credit transfer")))? {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                IdentityInsufficientBalanceError::new(self.identity_id(), balance, amount)
                    .into(),
            ));
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
