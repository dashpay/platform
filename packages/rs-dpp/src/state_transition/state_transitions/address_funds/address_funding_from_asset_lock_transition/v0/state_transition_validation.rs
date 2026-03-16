use crate::address_funds::AddressFundsFeeStrategyStep;
use crate::consensus::basic::overflow_error::OverflowError;
use crate::consensus::basic::state_transition::{
    FeeStrategyDuplicateError, FeeStrategyEmptyError, FeeStrategyIndexOutOfBoundsError,
    FeeStrategyTooManyStepsError, InputBelowMinimumError, InputWitnessCountMismatchError,
    InvalidRemainderOutputCountError, OutputAddressAlsoInputError, OutputBelowMinimumError,
    TransitionNoOutputsError, TransitionOverMaxInputsError, TransitionOverMaxOutputsError,
};
use crate::consensus::basic::BasicError;
use crate::state_transition::address_funding_from_asset_lock_transition::v0::AddressFundingFromAssetLockTransitionV0;
use crate::state_transition::StateTransitionStructureValidation;
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;
use std::collections::HashSet;

impl StateTransitionStructureValidation for AddressFundingFromAssetLockTransitionV0 {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        // Validate at least one output (asset lock must fund at least one address)
        if self.outputs.is_empty() {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::TransitionNoOutputsError(TransitionNoOutputsError::new()).into(),
            );
        }

        // Validate exactly one output has None value (remainder recipient)
        // This ensures full asset lock consumption - one address receives whatever is left
        let remainder_count = self.outputs.values().filter(|v| v.is_none()).count();
        if remainder_count != 1 {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::InvalidRemainderOutputCountError(
                    InvalidRemainderOutputCountError::new(
                        remainder_count.min(u16::MAX as usize) as u16
                    ),
                )
                .into(),
            );
        }

        // Validate maximum inputs (inputs are optional for combining with existing address funds)
        if self.inputs.len() > platform_version.dpp.state_transitions.max_address_inputs as usize {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::TransitionOverMaxInputsError(TransitionOverMaxInputsError::new(
                    self.inputs.len().min(u16::MAX as usize) as u16,
                    platform_version.dpp.state_transitions.max_address_inputs,
                ))
                .into(),
            );
        }

        // Validate maximum outputs
        if self.outputs.len() > platform_version.dpp.state_transitions.max_address_outputs as usize
        {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::TransitionOverMaxOutputsError(TransitionOverMaxOutputsError::new(
                    self.outputs.len().min(u16::MAX as usize) as u16,
                    platform_version.dpp.state_transitions.max_address_outputs,
                ))
                .into(),
            );
        }

        // Validate input witnesses count matches inputs count (if there are inputs)
        if self.inputs.len() != self.input_witnesses.len() {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::InputWitnessCountMismatchError(InputWitnessCountMismatchError::new(
                    self.inputs.len().min(u16::MAX as usize) as u16,
                    self.input_witnesses.len().min(u16::MAX as usize) as u16,
                ))
                .into(),
            );
        }

        // Validate no output address is also an input address
        for output_address in self.outputs.keys() {
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
                    if *index as usize >= self.outputs.len() {
                        return SimpleConsensusValidationResult::new_with_error(
                            BasicError::FeeStrategyIndexOutOfBoundsError(
                                FeeStrategyIndexOutOfBoundsError::new(
                                    "ReduceOutput",
                                    *index,
                                    self.outputs.len().min(u16::MAX as usize) as u16,
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

        // Validate each explicit output (Some value) is at least min_output_amount
        // The None output (remainder) will be computed at execution time
        for amount in self.outputs.values().flatten() {
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

        // Validate explicit outputs sum doesn't overflow
        let explicit_output_sum = self
            .outputs
            .values()
            .flatten()
            .try_fold(0u64, |acc, amount| acc.checked_add(*amount));
        if explicit_output_sum.is_none() {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::OverflowError(OverflowError::new("Output sum overflow".to_string()))
                    .into(),
            );
        }

        SimpleConsensusValidationResult::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address_funds::{AddressWitness, PlatformAddress};
    use crate::consensus::ConsensusError;
    use assert_matches::assert_matches;
    use std::collections::BTreeMap;

    /// Helper: returns a valid base transition that passes all validations.
    /// Has one input, one explicit output, and one remainder output.
    fn valid_asset_lock_transition() -> AddressFundingFromAssetLockTransitionV0 {
        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([1u8; 20]), (0, 1_000_000));

        let mut outputs = BTreeMap::new();
        // One explicit output and one remainder (None)
        outputs.insert(PlatformAddress::P2pkh([2u8; 20]), Some(500_000));
        outputs.insert(PlatformAddress::P2pkh([3u8; 20]), None);

        AddressFundingFromAssetLockTransitionV0 {
            inputs,
            outputs,
            fee_strategy: vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            user_fee_increase: 0,
            input_witnesses: vec![AddressWitness::P2pkh {
                signature: vec![0u8; 65].into(),
            }],
            ..Default::default()
        }
    }

    #[test]
    fn should_validate_a_valid_transition() {
        let platform_version = PlatformVersion::latest();
        let transition = valid_asset_lock_transition();
        let result = transition.validate_structure(platform_version);
        assert!(
            result.is_valid(),
            "Expected valid result, got errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn should_return_invalid_if_no_outputs() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_asset_lock_transition();
        transition.outputs.clear();

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::TransitionNoOutputsError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_if_zero_remainder_outputs() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_asset_lock_transition();
        // Replace all outputs with explicit (Some) values -- no remainder
        transition.outputs.clear();
        transition
            .outputs
            .insert(PlatformAddress::P2pkh([2u8; 20]), Some(500_000));

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::InvalidRemainderOutputCountError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_if_two_remainder_outputs() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_asset_lock_transition();
        transition.outputs.clear();
        transition
            .outputs
            .insert(PlatformAddress::P2pkh([2u8; 20]), None);
        transition
            .outputs
            .insert(PlatformAddress::P2pkh([3u8; 20]), None);

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::InvalidRemainderOutputCountError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_if_too_many_inputs() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_asset_lock_transition();
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
    fn should_return_invalid_if_too_many_outputs() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_asset_lock_transition();
        let max = platform_version.dpp.state_transitions.max_address_outputs;
        transition.outputs.clear();
        for i in 0..=(max as usize) {
            let mut hash = [0u8; 20];
            hash[0] = (i & 0xFF) as u8;
            hash[1] = ((i >> 8) & 0xFF) as u8;
            // Make exactly one None output (remainder) to pass that check first
            let value = if i == 0 { None } else { Some(500_000) };
            transition
                .outputs
                .insert(PlatformAddress::P2pkh(hash), value);
        }

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::TransitionOverMaxOutputsError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_if_witness_count_mismatch() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_asset_lock_transition();
        // Add an extra witness without a corresponding input
        transition.input_witnesses.push(AddressWitness::P2pkh {
            signature: vec![0u8; 65].into(),
        });

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
        let mut transition = valid_asset_lock_transition();
        let shared_addr = PlatformAddress::P2pkh([10u8; 20]);
        transition.inputs.clear();
        transition.inputs.insert(shared_addr, (0, 1_000_000));
        transition.outputs.clear();
        transition.outputs.insert(shared_addr, None);
        // Need a second output for remainder count check or same address serves both

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
        let mut transition = valid_asset_lock_transition();
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
        let mut transition = valid_asset_lock_transition();
        let max = platform_version
            .dpp
            .state_transitions
            .max_address_fee_strategies;
        transition.fee_strategy.clear();
        // Add more strategies than allowed. Use ReduceOutput since we have outputs.
        for i in 0..=(max as u16) {
            transition
                .fee_strategy
                .push(AddressFundsFeeStrategyStep::ReduceOutput(i));
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
        let mut transition = valid_asset_lock_transition();
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
        let mut transition = valid_asset_lock_transition();
        // Only 1 input (index 0), refer to index 5
        transition.fee_strategy = vec![AddressFundsFeeStrategyStep::DeductFromInput(5)];

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::FeeStrategyIndexOutOfBoundsError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_if_reduce_output_index_out_of_bounds() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_asset_lock_transition();
        // Only 2 outputs (indices 0 and 1), refer to index 10
        transition.fee_strategy = vec![AddressFundsFeeStrategyStep::ReduceOutput(10)];

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
        let mut transition = valid_asset_lock_transition();
        // Set input amount below minimum (100_000)
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
    fn should_return_invalid_if_explicit_output_below_minimum() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_asset_lock_transition();
        // Set explicit output amount below minimum (500_000)
        transition.outputs.clear();
        transition
            .outputs
            .insert(PlatformAddress::P2pkh([2u8; 20]), Some(1));
        transition
            .outputs
            .insert(PlatformAddress::P2pkh([3u8; 20]), None);

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::OutputBelowMinimumError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_if_explicit_output_sum_overflows() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_asset_lock_transition();
        transition.outputs.clear();
        transition
            .outputs
            .insert(PlatformAddress::P2pkh([2u8; 20]), Some(u64::MAX));
        transition
            .outputs
            .insert(PlatformAddress::P2pkh([3u8; 20]), Some(u64::MAX));
        transition
            .outputs
            .insert(PlatformAddress::P2pkh([4u8; 20]), None);

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(BasicError::OverflowError(_))]
        );
    }

    #[test]
    fn should_pass_with_no_inputs_and_valid_outputs() {
        let platform_version = PlatformVersion::latest();
        // Asset lock funding can have zero inputs (just the asset lock)
        let mut outputs = BTreeMap::new();
        outputs.insert(PlatformAddress::P2pkh([2u8; 20]), Some(500_000));
        outputs.insert(PlatformAddress::P2pkh([3u8; 20]), None);

        let transition = AddressFundingFromAssetLockTransitionV0 {
            inputs: BTreeMap::new(),
            outputs,
            fee_strategy: vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
            user_fee_increase: 0,
            input_witnesses: vec![],
            ..Default::default()
        };

        let result = transition.validate_structure(platform_version);
        assert!(
            result.is_valid(),
            "Expected valid result with no inputs, got errors: {:?}",
            result.errors
        );
    }
}
