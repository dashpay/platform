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
use crate::state_transition::address_credit_withdrawal_transition::{
    MIN_CORE_FEE_PER_BYTE, MIN_WITHDRAWAL_AMOUNT,
};
use crate::state_transition::StateTransitionStructureValidation;
use crate::util::is_non_zero_fibonacci_number::is_non_zero_fibonacci_number;
use crate::validation::SimpleConsensusValidationResult;
use crate::withdrawal::Pooling;
use platform_version::version::PlatformVersion;
use std::collections::HashSet;

impl StateTransitionStructureValidation for AddressCreditWithdrawalTransitionV0 {
    fn validate_structure(
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

        // Validate input witnesses count matches inputs count
        if self.inputs.len() != self.input_witnesses.len() {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::InputWitnessCountMismatchError(InputWitnessCountMismatchError::new(
                    self.inputs.len().min(u16::MAX as usize) as u16,
                    self.input_witnesses.len().min(u16::MAX as usize) as u16,
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
        if input_sum.is_none() {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::OverflowError(OverflowError::new("Input sum overflow".to_string()))
                    .into(),
            );
        }

        // Validate that input_sum > output_amount (withdrawal amount must be positive)
        let input_sum = input_sum.unwrap(); // Safe: checked above
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
        if withdrawal_amount < MIN_WITHDRAWAL_AMOUNT
            || withdrawal_amount > platform_version.system_limits.max_withdrawal_amount
        {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::WithdrawalBelowMinAmountError(WithdrawalBelowMinAmountError::new(
                    withdrawal_amount,
                    MIN_WITHDRAWAL_AMOUNT,
                    platform_version.system_limits.max_withdrawal_amount,
                ))
                .into(),
            );
        }

        SimpleConsensusValidationResult::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address_funds::AddressWitness;
    use crate::address_funds::PlatformAddress;
    use crate::identity::core_script::CoreScript;
    use assert_matches::assert_matches;
    use rand::SeedableRng;
    use std::collections::BTreeMap;

    fn valid_withdrawal_transition() -> AddressCreditWithdrawalTransitionV0 {
        use rand::SeedableRng;

        let input_amount = 1_000_000_000u64;
        AddressCreditWithdrawalTransitionV0 {
            inputs: [(PlatformAddress::P2pkh([1u8; 20]), (0, input_amount))].into(),
            input_witnesses: vec![AddressWitness::P2pkh {
                signature: vec![0u8; 65].into(),
            }],
            fee_strategy: vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            core_fee_per_byte: 1,
            output_script: CoreScript::random_p2pkh(&mut rand::rngs::StdRng::seed_from_u64(42)),
            pooling: Pooling::Never,
            output: None,
            user_fee_increase: 0,
        }
    }

    fn p2pkh_address(index: usize) -> PlatformAddress {
        let mut bytes = [0u8; 20];
        bytes[0] = (index & 0xff) as u8;
        bytes[1] = ((index >> 8) & 0xff) as u8;
        bytes[2] = ((index >> 16) & 0xff) as u8;
        bytes[3] = ((index >> 24) & 0xff) as u8;
        PlatformAddress::P2pkh(bytes)
    }

    fn make_inputs_and_witnesses(
        count: usize,
        amount: u64,
    ) -> (BTreeMap<PlatformAddress, (u32, u64)>, Vec<AddressWitness>) {
        let inputs = (0..count)
            .map(|i| (p2pkh_address(i), (i as u32, amount)))
            .collect();
        let witnesses = (0..count)
            .map(|_| AddressWitness::P2pkh {
                signature: vec![0u8; 65].into(),
            })
            .collect();
        (inputs, witnesses)
    }

    #[test]
    fn should_return_invalid_result_if_no_inputs() {
        let platform_version = PlatformVersion::latest();

        let mut transition = valid_withdrawal_transition();
        transition.inputs = BTreeMap::new();
        transition.input_witnesses = vec![];

        let result = transition.validate_structure(platform_version);

        assert_matches!(
            result.errors.as_slice(),
            [crate::consensus::ConsensusError::BasicError(
                BasicError::TransitionNoInputsError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_result_if_over_max_inputs() {
        let platform_version = PlatformVersion::latest();
        let max_inputs = platform_version.dpp.state_transitions.max_address_inputs as usize;

        let mut transition = valid_withdrawal_transition();
        let (inputs, witnesses) = make_inputs_and_witnesses(max_inputs + 1, 1_000_000_000);
        transition.inputs = inputs;
        transition.input_witnesses = witnesses;

        let result = transition.validate_structure(platform_version);

        assert_matches!(
            result.errors.as_slice(),
            [crate::consensus::ConsensusError::BasicError(
                BasicError::TransitionOverMaxInputsError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_result_if_witness_count_mismatch() {
        let platform_version = PlatformVersion::latest();

        let mut transition = valid_withdrawal_transition();
        transition.input_witnesses.push(AddressWitness::P2pkh {
            signature: vec![0u8; 65].into(),
        });

        let result = transition.validate_structure(platform_version);

        assert_matches!(
            result.errors.as_slice(),
            [crate::consensus::ConsensusError::BasicError(
                BasicError::InputWitnessCountMismatchError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_result_if_output_address_also_input() {
        let platform_version = PlatformVersion::latest();

        let mut transition = valid_withdrawal_transition();
        let input_address = transition
            .inputs
            .keys()
            .next()
            .expect("valid transition has one input")
            .clone();
        transition.output = Some((input_address, 1000));

        let result = transition.validate_structure(platform_version);

        assert_matches!(
            result.errors.as_slice(),
            [crate::consensus::ConsensusError::BasicError(
                BasicError::OutputAddressAlsoInputError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_result_if_fee_strategy_empty() {
        let platform_version = PlatformVersion::latest();

        let mut transition = valid_withdrawal_transition();
        transition.fee_strategy = vec![];

        let result = transition.validate_structure(platform_version);

        assert_matches!(
            result.errors.as_slice(),
            [crate::consensus::ConsensusError::BasicError(
                BasicError::FeeStrategyEmptyError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_result_if_fee_strategy_too_many_steps() {
        let platform_version = PlatformVersion::latest();
        let max_fee_strategies = platform_version
            .dpp
            .state_transitions
            .max_address_fee_strategies as usize;
        let step_count = max_fee_strategies + 1;

        let mut transition = valid_withdrawal_transition();
        let (inputs, witnesses) = make_inputs_and_witnesses(step_count, 1_000_000_000);
        transition.inputs = inputs;
        transition.input_witnesses = witnesses;
        transition.fee_strategy = (0..step_count)
            .map(|i| AddressFundsFeeStrategyStep::DeductFromInput(i as u16))
            .collect();

        let result = transition.validate_structure(platform_version);

        assert_matches!(
            result.errors.as_slice(),
            [crate::consensus::ConsensusError::BasicError(
                BasicError::FeeStrategyTooManyStepsError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_result_if_fee_strategy_duplicate() {
        let platform_version = PlatformVersion::latest();

        let mut transition = valid_withdrawal_transition();
        transition.fee_strategy = vec![
            AddressFundsFeeStrategyStep::DeductFromInput(0),
            AddressFundsFeeStrategyStep::DeductFromInput(0),
        ];

        let result = transition.validate_structure(platform_version);

        assert_matches!(
            result.errors.as_slice(),
            [crate::consensus::ConsensusError::BasicError(
                BasicError::FeeStrategyDuplicateError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_result_if_fee_strategy_index_out_of_bounds() {
        let platform_version = PlatformVersion::latest();

        let mut transition = valid_withdrawal_transition();
        transition.fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(999)];

        let result = transition.validate_structure(platform_version);

        assert_matches!(
            result.errors.as_slice(),
            [crate::consensus::ConsensusError::BasicError(
                BasicError::FeeStrategyIndexOutOfBoundsError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_result_if_input_below_minimum() {
        let platform_version = PlatformVersion::latest();

        let mut transition = valid_withdrawal_transition();
        transition.inputs = [(PlatformAddress::P2pkh([1u8; 20]), (0, 1))].into();

        let result = transition.validate_structure(platform_version);

        assert_matches!(
            result.errors.as_slice(),
            [crate::consensus::ConsensusError::BasicError(
                BasicError::InputBelowMinimumError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_result_if_output_below_minimum() {
        let platform_version = PlatformVersion::latest();

        let mut transition = valid_withdrawal_transition();
        transition.output = Some((PlatformAddress::P2pkh([2u8; 20]), 1));

        let result = transition.validate_structure(platform_version);

        assert_matches!(
            result.errors.as_slice(),
            [crate::consensus::ConsensusError::BasicError(
                BasicError::OutputBelowMinimumError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_result_if_pooling_not_never() {
        let platform_version = PlatformVersion::latest();

        let mut transition = valid_withdrawal_transition();
        transition.pooling = Pooling::IfAvailable;

        let result = transition.validate_structure(platform_version);

        assert_matches!(
            result.errors.as_slice(),
            [crate::consensus::ConsensusError::BasicError(
                BasicError::NotImplementedCreditWithdrawalTransitionPoolingError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_result_if_core_fee_not_fibonacci() {
        let platform_version = PlatformVersion::latest();

        let mut transition = valid_withdrawal_transition();
        transition.core_fee_per_byte = 4;

        let result = transition.validate_structure(platform_version);

        assert_matches!(
            result.errors.as_slice(),
            [crate::consensus::ConsensusError::BasicError(
                BasicError::InvalidCreditWithdrawalTransitionCoreFeeError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_result_if_output_script_invalid() {
        let platform_version = PlatformVersion::latest();

        let mut transition = valid_withdrawal_transition();
        transition.output_script = CoreScript::from_bytes(vec![0x51]);

        let result = transition.validate_structure(platform_version);

        assert_matches!(
            result.errors.as_slice(),
            [crate::consensus::ConsensusError::BasicError(
                BasicError::InvalidCreditWithdrawalTransitionOutputScriptError(_)
            )]
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
            core_fee_per_byte: 1, // Valid Fibonacci number — ensures we reach the overflow check
            output_script: CoreScript::random_p2pkh(&mut rand::rngs::StdRng::seed_from_u64(1)),
            ..Default::default()
        };

        let result = transition.validate_structure(platform_version);

        assert_matches!(
            result.errors.as_slice(),
            [crate::consensus::ConsensusError::BasicError(
                BasicError::OverflowError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_result_if_withdrawal_below_min() {
        let platform_version = PlatformVersion::latest();
        let min_input_amount = platform_version
            .dpp
            .state_transitions
            .address_funds
            .min_input_amount;
        assert!(min_input_amount < MIN_WITHDRAWAL_AMOUNT);

        let mut transition = valid_withdrawal_transition();
        transition.inputs = [(PlatformAddress::P2pkh([1u8; 20]), (0, min_input_amount))].into();
        transition.output = None;

        let result = transition.validate_structure(platform_version);

        assert_matches!(
            result.errors.as_slice(),
            [crate::consensus::ConsensusError::BasicError(
                BasicError::WithdrawalBelowMinAmountError(_)
            )]
        );
    }

    #[test]
    fn should_return_valid_result_for_valid_transition() {
        let platform_version = PlatformVersion::latest();
        let transition = valid_withdrawal_transition();

        let result = transition.validate_structure(platform_version);

        assert_matches!(result.errors.as_slice(), []);
    }
}
