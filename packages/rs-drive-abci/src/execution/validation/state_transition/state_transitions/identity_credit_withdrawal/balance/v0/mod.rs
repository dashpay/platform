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

        let amount_and_fees = self.amount()
            .checked_add(platform_version.fee_version.state_transition_min_fees.credit_withdrawal)
            .ok_or_else(|| Error::Execution(ExecutionError::Overflow("overflow when adding amount and min_leftover_credits_before_processing in identity credit withdrawal")))?;

        if balance < amount_and_fees {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                IdentityInsufficientBalanceError::new(self.identity_id(), balance, self.amount())
                    .into(),
            ));
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use assert_matches::assert_matches;
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;
    use dpp::prelude::Identifier;
    use dpp::state_transition::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0;
    use platform_version::version::v1::PLATFORM_V1;

    mod validate_advanced_minimum_balance_pre_check_v0 {
        use super::*;

        #[test]
        fn should_return_invalid_result_if_balance_is_less_than_amount_and_fees() {
            let balance = 100;

            let amount = 200;

            let identity = PartialIdentity {
                id: Identifier::random(),
                loaded_public_keys: Default::default(),
                balance: Some(balance),
                revision: None,
                not_found_public_keys: Default::default(),
            };

            let transaction =
                IdentityCreditWithdrawalTransition::V0(IdentityCreditWithdrawalTransitionV0 {
                    identity_id: identity.id,
                    amount,
                    core_fee_per_byte: 0,
                    pooling: Default::default(),
                    output_script: Default::default(),
                    nonce: 0,
                    user_fee_increase: 0,
                    signature_public_key_id: 0,
                    signature: Default::default(),
                });

            let platform_version = &PLATFORM_V1;

            let result = transaction
                .validate_advanced_minimum_balance_pre_check_v0(&identity, platform_version)
                .expect("failed to validate minimum balance");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::IdentityInsufficientBalanceError(err)
                )] if err.balance() == balance && err.required_balance() == amount && err.identity_id() == &identity.id
            );
        }

        #[test]
        fn should_return_valid_result() {
            let balance = 200000000000;

            let amount = 100;

            let identity = PartialIdentity {
                id: Identifier::random(),
                loaded_public_keys: Default::default(),
                balance: Some(balance),
                revision: None,
                not_found_public_keys: Default::default(),
            };

            let transaction =
                IdentityCreditWithdrawalTransition::V0(IdentityCreditWithdrawalTransitionV0 {
                    identity_id: Default::default(),
                    amount,
                    core_fee_per_byte: 0,
                    pooling: Default::default(),
                    output_script: Default::default(),
                    nonce: 0,
                    user_fee_increase: 0,
                    signature_public_key_id: 0,
                    signature: Default::default(),
                });

            let platform_version = &PLATFORM_V1;

            let result = transaction
                .validate_advanced_minimum_balance_pre_check_v0(&identity, platform_version)
                .expect("failed to validate minimum balance");

            assert_matches!(result.errors.as_slice(), []);
        }
    }
}
