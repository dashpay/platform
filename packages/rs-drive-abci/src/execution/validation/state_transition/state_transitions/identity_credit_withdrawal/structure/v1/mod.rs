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
use dpp::version::PlatformVersion;
use dpp::withdrawal::Pooling;

pub(in crate::execution::validation::state_transition::state_transitions::identity_credit_withdrawal) trait IdentityCreditWithdrawalStateTransitionStructureValidationV1 {
    fn validate_basic_structure_v1(&self, platform_version: &PlatformVersion) -> Result<SimpleConsensusValidationResult, Error>;
}

impl IdentityCreditWithdrawalStateTransitionStructureValidationV1
    for IdentityCreditWithdrawalTransition
{
    fn validate_basic_structure_v1(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let mut result = SimpleConsensusValidationResult::default();

        let amount = self.amount();
        if amount < MIN_WITHDRAWAL_AMOUNT
            || amount > platform_version.system_limits.max_withdrawal_amount
        {
            result.add_error(ConsensusError::from(
                InvalidIdentityCreditWithdrawalTransitionAmountError::new(
                    self.amount(),
                    MIN_WITHDRAWAL_AMOUNT,
                    platform_version.system_limits.max_withdrawal_amount,
                ),
            ));
        }

        // Now, we do not support pooling, so we must validate that pooling is `Never`

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

#[cfg(test)]
mod tests {
    use super::*;

    use assert_matches::assert_matches;
    use dpp::consensus::basic::BasicError;
    use dpp::dashcore::ScriptBuf;
    use dpp::identity::core_script::CoreScript;
    use dpp::state_transition::identity_credit_withdrawal_transition::v1::IdentityCreditWithdrawalTransitionV1;
    use platform_version::version::v1::PLATFORM_V1;

    mod validate_basic_structure_v1 {
        use super::*;

        #[test]
        fn should_return_invalid_result_if_amount_too_low() {
            let amount = 18000;

            let transition =
                IdentityCreditWithdrawalTransition::V1(IdentityCreditWithdrawalTransitionV1 {
                    identity_id: Default::default(),
                    amount,
                    core_fee_per_byte: 1,
                    pooling: Default::default(),
                    output_script: None,
                    nonce: 0,
                    user_fee_increase: 0,
                    signature_public_key_id: 0,
                    signature: Default::default(),
                });

            let platform_version = &PLATFORM_V1;

            let result = transition
                .validate_basic_structure_v1(platform_version)
                .expect("failed to validate basic structure");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::InvalidIdentityCreditWithdrawalTransitionAmountError(
                        InvalidIdentityCreditWithdrawalTransitionAmountError {
                            amount: a,
                            min_amount: 190000,
                            max_amount: 50000000000000,
                        },
                    ),
                )] if *a == amount
            );
        }

        #[test]
        fn should_return_invalid_result_if_amount_too_high() {
            let amount = 60000000000000;

            let transition =
                IdentityCreditWithdrawalTransition::V1(IdentityCreditWithdrawalTransitionV1 {
                    identity_id: Default::default(),
                    amount,
                    core_fee_per_byte: 1,
                    pooling: Default::default(),
                    output_script: None,
                    nonce: 0,
                    user_fee_increase: 0,
                    signature_public_key_id: 0,
                    signature: Default::default(),
                });

            let platform_version = &PLATFORM_V1;

            let result = transition
                .validate_basic_structure_v1(platform_version)
                .expect("failed to validate basic structure");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::InvalidIdentityCreditWithdrawalTransitionAmountError(
                        InvalidIdentityCreditWithdrawalTransitionAmountError {
                            amount: a,
                            min_amount: 190000,
                            max_amount: 50000000000000,
                        },
                    ),
                )] if *a == amount
            );
        }

        #[test]
        fn should_return_invalid_result_if_pooling_not_never() {
            let transition =
                IdentityCreditWithdrawalTransition::V1(IdentityCreditWithdrawalTransitionV1 {
                    identity_id: Default::default(),
                    amount: 200000,
                    core_fee_per_byte: 1,
                    pooling: Pooling::Standard,
                    output_script: None,
                    nonce: 0,
                    user_fee_increase: 0,
                    signature_public_key_id: 0,
                    signature: Default::default(),
                });

            let platform_version = &PLATFORM_V1;

            let result = transition
                .validate_basic_structure_v1(platform_version)
                .expect("failed to validate basic structure");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::NotImplementedIdentityCreditWithdrawalTransitionPoolingError(err),
                )] if err.pooling() == Pooling::Standard as u8
            );
        }

        #[test]
        fn should_return_invalid_result_if_core_fee_not_fibonacci() {
            let transition =
                IdentityCreditWithdrawalTransition::V1(IdentityCreditWithdrawalTransitionV1 {
                    identity_id: Default::default(),
                    amount: 200000,
                    core_fee_per_byte: 0,
                    pooling: Pooling::Never,
                    output_script: None,
                    nonce: 0,
                    user_fee_increase: 0,
                    signature_public_key_id: 0,
                    signature: Default::default(),
                });

            let platform_version = &PLATFORM_V1;

            let result = transition
                .validate_basic_structure_v1(platform_version)
                .expect("failed to validate basic structure");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::InvalidIdentityCreditWithdrawalTransitionCoreFeeError(err)
                )] if err.min_core_fee_per_byte() == 1 && err.core_fee_per_byte() == 0
            );
        }

        #[test]
        fn should_return_invalid_result_if_output_script_is_not_p2pkh_or_p2sh() {
            let output_script = CoreScript::new(ScriptBuf::new());

            let transition =
                IdentityCreditWithdrawalTransition::V1(IdentityCreditWithdrawalTransitionV1 {
                    identity_id: Default::default(),
                    amount: 200000,
                    core_fee_per_byte: 1,
                    pooling: Pooling::Never,
                    output_script: Some(output_script.clone()),
                    nonce: 0,
                    user_fee_increase: 0,
                    signature_public_key_id: 0,
                    signature: Default::default(),
                });

            let platform_version = &PLATFORM_V1;

            let result = transition
                .validate_basic_structure_v1(platform_version)
                .expect("failed to validate basic structure");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::InvalidIdentityCreditWithdrawalTransitionOutputScriptError(err)
                )] if err.output_script() == output_script
            );
        }

        #[test]
        fn should_return_valid_result_if_output_script_is_p2pkh() {
            let transition =
                IdentityCreditWithdrawalTransition::V1(IdentityCreditWithdrawalTransitionV1 {
                    identity_id: Default::default(),
                    amount: 200000,
                    core_fee_per_byte: 1,
                    pooling: Pooling::Never,
                    output_script: None,
                    nonce: 0,
                    user_fee_increase: 0,
                    signature_public_key_id: 0,
                    signature: Default::default(),
                });

            let platform_version = &PLATFORM_V1;

            let result = transition
                .validate_basic_structure_v1(platform_version)
                .expect("failed to validate basic structure");

            assert!(result.is_valid());
        }

        #[test]
        fn should_return_valid_result_if_output_script_is_p2sh() {
            let transition =
                IdentityCreditWithdrawalTransition::V1(IdentityCreditWithdrawalTransitionV1 {
                    identity_id: Default::default(),
                    amount: 200000,
                    core_fee_per_byte: 1,
                    pooling: Pooling::Never,
                    output_script: None,
                    nonce: 0,
                    user_fee_increase: 0,
                    signature_public_key_id: 0,
                    signature: Default::default(),
                });

            let platform_version = &PLATFORM_V1;

            let result = transition
                .validate_basic_structure_v1(platform_version)
                .expect("failed to validate basic structure");

            assert!(result.is_valid());
        }
    }
}
