use dpp::consensus::basic::identity::{
    InvalidIdentityCreditWithdrawalTransitionAmountError,
    InvalidIdentityCreditWithdrawalTransitionCoreFeeError,
    InvalidIdentityCreditWithdrawalTransitionOutputScriptError,
    NotImplementedIdentityCreditWithdrawalTransitionPoolingError,
};
use dpp::consensus::ConsensusError;

use crate::error::Error;
use dpp::state_transition::identity_credit_withdrawal_transition::accessors::IdentityCreditWithdrawalTransitionAccessorsV0;
use dpp::state_transition::identity_credit_withdrawal_transition::{
    IdentityCreditWithdrawalTransition, MIN_CORE_FEE_PER_BYTE, MIN_WITHDRAWAL_AMOUNT,
};
use dpp::util::is_fibonacci_number::is_fibonacci_number;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::withdrawal::Pooling;

pub(in crate::execution::validation::state_transition::state_transitions::identity_credit_withdrawal) trait IdentityCreditWithdrawalStateTransitionStructureValidationV1 {
    fn validate_basic_structure_v1(&self) -> Result<SimpleConsensusValidationResult, Error>;
}

impl IdentityCreditWithdrawalStateTransitionStructureValidationV1
    for IdentityCreditWithdrawalTransition
{
    fn validate_basic_structure_v1(&self) -> Result<SimpleConsensusValidationResult, Error> {
        let mut result = SimpleConsensusValidationResult::default();

        if self.amount() < MIN_WITHDRAWAL_AMOUNT {
            result.add_error(ConsensusError::from(
                InvalidIdentityCreditWithdrawalTransitionAmountError::new(
                    self.amount(),
                    MIN_WITHDRAWAL_AMOUNT,
                ),
            ));
        }

        // currently we do not support pooling, so we must validate that pooling is `Never`

        if self.pooling() != Pooling::Never {
            result.add_error(
                NotImplementedIdentityCreditWithdrawalTransitionPoolingError::new(
                    self.pooling() as u8
                ),
            );

            return Ok(result);
        }

        // validate core_fee is in fibonacci sequence
        if !is_fibonacci_number(self.core_fee_per_byte() as u64) {
            result.add_error(InvalidIdentityCreditWithdrawalTransitionCoreFeeError::new(
                self.core_fee_per_byte(),
                MIN_CORE_FEE_PER_BYTE,
            ));

            return Ok(result);
        }

        if let Some(output_script) = self.output_script() {
            // validate output_script types
            if !output_script.is_p2pkh() && !output_script.is_p2sh() {
                result.add_error(
                    InvalidIdentityCreditWithdrawalTransitionOutputScriptError::new(
                        output_script.clone(),
                    ),
                );
            }
        }

        Ok(result)
    }
}
