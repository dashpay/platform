use crate::error::Error;
use dpp::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

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
        // Delegate to the shared DPP-owned v1 basic-structure rule so the
        // server and the SDK constructor cannot drift apart. The DPP method
        // owns the amount/pooling/core-fee/output-script logic; this trait
        // method only adapts its return type for drive-abci's error chain.
        Ok(self.basic_structure_rules_v1(platform_version))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use assert_matches::assert_matches;
    use dpp::consensus::basic::identity::InvalidIdentityCreditWithdrawalTransitionAmountError;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::ConsensusError;
    use dpp::dashcore::ScriptBuf;
    use dpp::identity::core_script::CoreScript;
    use dpp::state_transition::identity_credit_withdrawal_transition::v1::IdentityCreditWithdrawalTransitionV1;
    use dpp::withdrawal::Pooling;
    use platform_version::version::v1::PLATFORM_V1;
    use rand::SeedableRng;

    mod validate_basic_structure_v1 {
        use super::*;
        use rand::prelude::StdRng;

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
                    BasicError::NotImplementedCreditWithdrawalTransitionPoolingError(err),
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
                    BasicError::InvalidCreditWithdrawalTransitionCoreFeeError(err)
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
                    BasicError::InvalidCreditWithdrawalTransitionOutputScriptError(err)
                )] if err.output_script() == output_script
            );
        }

        #[test]
        fn should_return_valid_result_if_output_script_is_p2pkh() {
            let rng = &mut StdRng::from_entropy();

            let transition =
                IdentityCreditWithdrawalTransition::V1(IdentityCreditWithdrawalTransitionV1 {
                    identity_id: Default::default(),
                    amount: 200000,
                    core_fee_per_byte: 1,
                    pooling: Pooling::Never,
                    output_script: Some(CoreScript::random_p2pkh(rng)),
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
            let rng = &mut StdRng::from_entropy();

            let transition =
                IdentityCreditWithdrawalTransition::V1(IdentityCreditWithdrawalTransitionV1 {
                    identity_id: Default::default(),
                    amount: 200000,
                    core_fee_per_byte: 1,
                    pooling: Pooling::Never,
                    output_script: Some(CoreScript::random_p2sh(rng)),
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
        fn should_return_valid_result_without_output_script() {
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
