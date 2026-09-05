use crate::address_funds::AddressFundsFeeStrategyStep;
use crate::consensus::basic::identity::{
    InvalidCreditWithdrawalTransitionCoreFeeError,
    InvalidCreditWithdrawalTransitionOutputScriptError,
    NotImplementedCreditWithdrawalTransitionPoolingError,
};
use crate::consensus::basic::overflow_error::OverflowError;
use crate::consensus::basic::state_transition::{
    FeeStrategyDuplicateError, FeeStrategyEmptyError, FeeStrategyIndexOutOfBoundsError,
    FeeStrategyTooManyStepsError, InputBelowMinimumError, InputWitnessCountMismatchError,
    OutputAddressAlsoInputError, OutputBelowMinimumError, TransitionNoInputsError,
    TransitionOverMaxInputsError, WithdrawalBalanceMismatchError, WithdrawalBelowMinAmountError,
};
use crate::consensus::basic::BasicError;
use crate::state_transition::address_credit_withdrawal_transition::v0::AddressCreditWithdrawalTransitionV0;
use crate::state_transition::address_credit_withdrawal_transition::MIN_CORE_FEE_PER_BYTE;
use crate::state_transition::StateTransitionStructureValidation;
use crate::util::is_non_zero_fibonacci_number::is_non_zero_fibonacci_number;
use crate::validation::SimpleConsensusValidationResult;
use crate::withdrawal::Pooling;
use platform_version::version::PlatformVersion;
use std::collections::HashSet;

impl AddressCreditWithdrawalTransitionV0 {
    /// Validates all structural properties of the transition except for the
    /// `input_witnesses` count. This is intended for client-side pre-signing
    /// validation, where witnesses are not yet present.
    pub fn validate_structure_without_input_witnesses(
        &self,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        // Validate at least one input
        if self.inputs.is_empty() {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::TransitionNoInputsError(TransitionNoInputsError::new()).into(),
            );
        }

        // Validate maximum inputs
        if self.inputs.len() > platform_version.dpp.state_transitions.max_address_inputs as usize {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::TransitionOverMaxInputsError(TransitionOverMaxInputsError::new(
                    self.inputs.len().min(u16::MAX as usize) as u16,
                    platform_version.dpp.state_transitions.max_address_inputs,
                ))
                .into(),
            );
        }

        // Validate output address is not also an input address
        if let Some((output_address, _)) = &self.output {
            if self.inputs.contains_key(output_address) {
                return SimpleConsensusValidationResult::new_with_error(
                    BasicError::OutputAddressAlsoInputError(OutputAddressAlsoInputError::new())
                        .into(),
                );
            }
        }

        // Validate fee strategy is not empty
        if self.fee_strategy.is_empty() {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::FeeStrategyEmptyError(FeeStrategyEmptyError::new()).into(),
            );
        }

        // Validate fee strategy has at most max_address_fee_strategies steps
        let max_fee_strategies = platform_version
            .dpp
            .state_transitions
            .max_address_fee_strategies as usize;
        if self.fee_strategy.len() > max_fee_strategies {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::FeeStrategyTooManyStepsError(FeeStrategyTooManyStepsError::new(
                    self.fee_strategy.len().min(u8::MAX as usize) as u8,
                    max_fee_strategies.min(u8::MAX as usize) as u8,
                ))
                .into(),
            );
        }

        // Validate fee strategy has no duplicates
        let mut seen = HashSet::with_capacity(self.fee_strategy.len());
        for step in &self.fee_strategy {
            if !seen.insert(step) {
                return SimpleConsensusValidationResult::new_with_error(
                    BasicError::FeeStrategyDuplicateError(FeeStrategyDuplicateError::new()).into(),
                );
            }
        }

        // Calculate number of outputs (0 or 1 for optional output)
        let output_count = if self.output.is_some() { 1 } else { 0 };

        // Validate fee strategy indices are within bounds
        for step in &self.fee_strategy {
            match step {
                AddressFundsFeeStrategyStep::DeductFromInput(index) => {
                    if *index as usize >= self.inputs.len() {
                        return SimpleConsensusValidationResult::new_with_error(
                            BasicError::FeeStrategyIndexOutOfBoundsError(
                                FeeStrategyIndexOutOfBoundsError::new(
                                    "DeductFromInput",
                                    *index,
                                    self.inputs.len().min(u16::MAX as usize) as u16,
                                ),
                            )
                            .into(),
                        );
                    }
                }
                AddressFundsFeeStrategyStep::ReduceOutput(index) => {
                    if *index as usize >= output_count {
                        return SimpleConsensusValidationResult::new_with_error(
                            BasicError::FeeStrategyIndexOutOfBoundsError(
                                FeeStrategyIndexOutOfBoundsError::new(
                                    "ReduceOutput",
                                    *index,
                                    output_count as u16,
                                ),
                            )
                            .into(),
                        );
                    }
                }
            }
        }

        let min_input_amount = platform_version
            .dpp
            .state_transitions
            .address_funds
            .min_input_amount;
        let min_output_amount = platform_version
            .dpp
            .state_transitions
            .address_funds
            .min_output_amount;

        // Validate each input is at least min_input_amount
        for (_nonce, amount) in self.inputs.values() {
            if *amount < min_input_amount {
                return SimpleConsensusValidationResult::new_with_error(
                    BasicError::InputBelowMinimumError(InputBelowMinimumError::new(
                        *amount,
                        min_input_amount,
                    ))
                    .into(),
                );
            }
        }

        // Validate output is at least min_output_amount (if present)
        if let Some((_, amount)) = &self.output {
            if *amount < min_output_amount {
                return SimpleConsensusValidationResult::new_with_error(
                    BasicError::OutputBelowMinimumError(OutputBelowMinimumError::new(
                        *amount,
                        min_output_amount,
                    ))
                    .into(),
                );
            }
        }

        // Validate pooling - currently we do not support pooling, so we must validate that pooling is `Never`
        if self.pooling != Pooling::Never {
            return SimpleConsensusValidationResult::new_with_error(
                NotImplementedCreditWithdrawalTransitionPoolingError::new(self.pooling as u8)
                    .into(),
            );
        }

        // Validate core_fee_per_byte is a Fibonacci number
        if !is_non_zero_fibonacci_number(self.core_fee_per_byte as u64) {
            return SimpleConsensusValidationResult::new_with_error(
                InvalidCreditWithdrawalTransitionCoreFeeError::new(
                    self.core_fee_per_byte,
                    MIN_CORE_FEE_PER_BYTE,
                )
                .into(),
            );
        }

        // Validate output_script is P2PKH or P2SH
        if !self.output_script.is_p2pkh() && !self.output_script.is_p2sh() {
            return SimpleConsensusValidationResult::new_with_error(
                InvalidCreditWithdrawalTransitionOutputScriptError::new(self.output_script.clone())
                    .into(),
            );
        }
        // Validate input sum doesn't overflow
        let input_sum = self
            .inputs
            .values()
            .try_fold(0u64, |acc, (_, amount)| acc.checked_add(*amount));
        let Some(input_sum) = input_sum else {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::OverflowError(OverflowError::new("Input sum overflow".to_string()))
                    .into(),
            );
        };

        // Validate that input_sum > output_amount (withdrawal amount must be positive)
        let output_amount = self.output.as_ref().map_or(0, |(_, amount)| *amount);
        if input_sum <= output_amount {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::WithdrawalBalanceMismatchError(WithdrawalBalanceMismatchError::new(
                    input_sum,
                    output_amount,
                    input_sum.saturating_sub(output_amount),
                ))
                .into(),
            );
        }

        // Validate withdrawal amount meets minimum and maximum
        let withdrawal_amount = input_sum - output_amount; // Safe: checked input_sum > output_amount above
        if withdrawal_amount < platform_version.system_limits.min_withdrawal_amount
            || withdrawal_amount > platform_version.system_limits.max_withdrawal_amount
        {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::WithdrawalBelowMinAmountError(WithdrawalBelowMinAmountError::new(
                    withdrawal_amount,
                    platform_version.system_limits.min_withdrawal_amount,
                    platform_version.system_limits.max_withdrawal_amount,
                ))
                .into(),
            );
        }

        SimpleConsensusValidationResult::new()
    }

    /// Validates that the number of `input_witnesses` matches the number of
    /// inputs. Intended to be invoked after signing, once witnesses have been
    /// produced by the signer.
    pub fn validate_input_witnesses_count(&self) -> SimpleConsensusValidationResult {
        if self.inputs.len() != self.input_witnesses.len() {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::InputWitnessCountMismatchError(InputWitnessCountMismatchError::new(
                    self.inputs.len().min(u16::MAX as usize) as u16,
                    self.input_witnesses.len().min(u16::MAX as usize) as u16,
                ))
                .into(),
            );
        }
        SimpleConsensusValidationResult::new()
    }
}

impl StateTransitionStructureValidation for AddressCreditWithdrawalTransitionV0 {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        let result = self.validate_structure_without_input_witnesses(platform_version);
        if !result.is_valid() {
            return result;
        }
        self.validate_input_witnesses_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address_funds::{AddressWitness, PlatformAddress};
    use crate::consensus::ConsensusError;
    use crate::identity::core_script::CoreScript;
    use assert_matches::assert_matches;
    use rand::SeedableRng;
    use std::collections::BTreeMap;

    /// Helper: returns a valid credit withdrawal transition that passes all validations.
    /// Uses an input large enough to cover the minimum withdrawal amount.
    fn valid_withdrawal_transition() -> AddressCreditWithdrawalTransitionV0 {
        let mut inputs = BTreeMap::new();
        // Need at least the v12 min_withdrawal_amount (1_000_000) for a valid withdrawal;
        // use 2_000_000 to sit comfortably above the floor rather than on its boundary.
        inputs.insert(PlatformAddress::P2pkh([1u8; 20]), (0, 2_000_000));

        AddressCreditWithdrawalTransitionV0 {
            inputs,
            output: None,
            fee_strategy: vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            core_fee_per_byte: 1, // 1 is a fibonacci number
            pooling: Pooling::Never,
            output_script: CoreScript::new_p2pkh([5u8; 20]),
            user_fee_increase: 0,
            input_witnesses: vec![AddressWitness::P2pkh {
                signature: vec![0u8; 65].into(),
            }],
        }
    }

    #[test]
    fn should_validate_a_valid_transition() {
        let platform_version = PlatformVersion::latest();
        let transition = valid_withdrawal_transition();
        let result = transition.validate_structure(platform_version);
        assert!(
            result.is_valid(),
            "Expected valid result, got errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn should_return_invalid_if_no_inputs() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_withdrawal_transition();
        transition.inputs.clear();
        transition.input_witnesses.clear();

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::TransitionNoInputsError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_if_too_many_inputs() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_withdrawal_transition();
        let max = platform_version.dpp.state_transitions.max_address_inputs;
        transition.inputs.clear();
        transition.input_witnesses.clear();
        for i in 0..=(max as usize) {
            let mut hash = [0u8; 20];
            hash[0] = (i & 0xFF) as u8;
            hash[1] = ((i >> 8) & 0xFF) as u8;
            transition
                .inputs
                .insert(PlatformAddress::P2pkh(hash), (0, 1_000_000));
            transition.input_witnesses.push(AddressWitness::P2pkh {
                signature: vec![0u8; 65].into(),
            });
        }

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::TransitionOverMaxInputsError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_if_witness_count_mismatch() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_withdrawal_transition();
        transition.input_witnesses.clear();

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::InputWitnessCountMismatchError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_if_output_address_also_input() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_withdrawal_transition();
        let shared_addr = PlatformAddress::P2pkh([1u8; 20]);
        transition.output = Some((shared_addr, 500_000));

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::OutputAddressAlsoInputError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_if_fee_strategy_empty() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_withdrawal_transition();
        transition.fee_strategy.clear();

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::FeeStrategyEmptyError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_if_fee_strategy_too_many_steps() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_withdrawal_transition();
        let max = platform_version
            .dpp
            .state_transitions
            .max_address_fee_strategies;
        transition.fee_strategy.clear();
        transition.inputs.clear();
        transition.input_witnesses.clear();
        for i in 0..=(max as usize) {
            let mut hash = [0u8; 20];
            hash[0] = (i & 0xFF) as u8;
            hash[1] = ((i >> 8) & 0xFF) as u8;
            transition
                .inputs
                .insert(PlatformAddress::P2pkh(hash), (0, 1_000_000));
            transition.input_witnesses.push(AddressWitness::P2pkh {
                signature: vec![0u8; 65].into(),
            });
            transition
                .fee_strategy
                .push(AddressFundsFeeStrategyStep::DeductFromInput(i as u16));
        }

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::FeeStrategyTooManyStepsError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_if_fee_strategy_has_duplicates() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_withdrawal_transition();
        transition.fee_strategy = vec![
            AddressFundsFeeStrategyStep::DeductFromInput(0),
            AddressFundsFeeStrategyStep::DeductFromInput(0),
        ];

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::FeeStrategyDuplicateError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_if_deduct_from_input_index_out_of_bounds() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_withdrawal_transition();
        transition.fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(99)];

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::FeeStrategyIndexOutOfBoundsError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_if_reduce_output_index_out_of_bounds_with_no_output() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_withdrawal_transition();
        transition.output = None;
        transition.fee_strategy = vec![AddressFundsFeeStrategyStep::ReduceOutput(0)];

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::FeeStrategyIndexOutOfBoundsError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_if_input_below_minimum() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_withdrawal_transition();
        transition.inputs.clear();
        transition
            .inputs
            .insert(PlatformAddress::P2pkh([1u8; 20]), (0, 1));

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::InputBelowMinimumError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_if_output_below_minimum() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_withdrawal_transition();
        // Need enough input to cover output + withdrawal amount
        transition.inputs.clear();
        transition
            .inputs
            .insert(PlatformAddress::P2pkh([1u8; 20]), (0, 10_000_000));
        transition.output = Some((PlatformAddress::P2pkh([9u8; 20]), 1));

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::OutputBelowMinimumError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_if_pooling_not_never() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_withdrawal_transition();
        transition.pooling = Pooling::IfAvailable;

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::NotImplementedCreditWithdrawalTransitionPoolingError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_if_core_fee_not_fibonacci() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_withdrawal_transition();
        transition.core_fee_per_byte = 4;

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::InvalidCreditWithdrawalTransitionCoreFeeError(_)
            )]
        );
    }

    #[test]
    fn should_accept_valid_fibonacci_core_fees() {
        let platform_version = PlatformVersion::latest();
        let fibonacci_numbers = [1, 2, 3, 5, 8, 13, 21];
        for fee in fibonacci_numbers {
            let mut transition = valid_withdrawal_transition();
            transition.core_fee_per_byte = fee;
            let result = transition.validate_structure(platform_version);
            assert!(
                result.is_valid(),
                "Expected valid for fibonacci fee {}, got errors: {:?}",
                fee,
                result.errors
            );
        }
    }

    #[test]
    fn should_return_invalid_if_output_script_not_p2pkh_or_p2sh() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_withdrawal_transition();
        transition.output_script = CoreScript::from_bytes(vec![0x00, 0x14, 0x01, 0x02]);

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::InvalidCreditWithdrawalTransitionOutputScriptError(_)
            )]
        );
    }

    #[test]
    fn should_accept_valid_p2sh_output_script() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_withdrawal_transition();
        transition.output_script = CoreScript::new_p2sh([7u8; 20]);

        let result = transition.validate_structure(platform_version);
        assert!(
            result.is_valid(),
            "Expected valid with P2SH output script, got errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn should_return_invalid_result_if_input_sum_overflows() {
        let platform_version = PlatformVersion::latest();

        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([1u8; 20]), (0, u64::MAX));
        inputs.insert(PlatformAddress::P2pkh([2u8; 20]), (0, u64::MAX));

        let transition = AddressCreditWithdrawalTransitionV0 {
            inputs,
            input_witnesses: vec![
                AddressWitness::P2pkh {
                    signature: vec![0u8; 65].into(),
                },
                AddressWitness::P2pkh {
                    signature: vec![0u8; 65].into(),
                },
            ],
            fee_strategy: vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            core_fee_per_byte: 1,
            output_script: CoreScript::random_p2pkh(&mut rand::rngs::StdRng::seed_from_u64(1)),
            ..Default::default()
        };

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(BasicError::OverflowError(_))]
        );
    }

    #[test]
    fn should_return_invalid_if_withdrawal_amount_below_minimum() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_withdrawal_transition();
        // Set input below the v12 min_withdrawal_amount (1_000_000) so the resulting
        // withdrawal is rejected as too small.
        transition.inputs.clear();
        transition
            .inputs
            .insert(PlatformAddress::P2pkh([1u8; 20]), (0, 100_000));

        let result = transition.validate_structure(platform_version);
        // 100_000 withdrawal < min_withdrawal_amount (1_000_000), should fail
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::WithdrawalBelowMinAmountError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_if_input_sum_not_greater_than_output() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_withdrawal_transition();
        // Set input and output equal -- no withdrawal amount
        transition.inputs.clear();
        transition
            .inputs
            .insert(PlatformAddress::P2pkh([1u8; 20]), (0, 1_000_000));
        transition.output = Some((PlatformAddress::P2pkh([9u8; 20]), 1_000_000));

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::WithdrawalBalanceMismatchError(_)
            )]
        );
    }

    #[test]
    fn should_validate_with_output_change() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_withdrawal_transition();
        // Large enough input to cover both output change and min withdrawal
        transition.inputs.clear();
        transition
            .inputs
            .insert(PlatformAddress::P2pkh([1u8; 20]), (0, 5_000_000));
        transition.output = Some((PlatformAddress::P2pkh([9u8; 20]), 500_000));
        transition.fee_strategy = vec![AddressFundsFeeStrategyStep::ReduceOutput(0)];

        let result = transition.validate_structure(platform_version);
        assert!(
            result.is_valid(),
            "Expected valid with output change, got errors: {:?}",
            result.errors
        );
    }
}
