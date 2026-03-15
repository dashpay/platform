use crate::address_funds::AddressFundsFeeStrategyStep;
use crate::consensus::basic::overflow_error::OverflowError;
use crate::consensus::basic::state_transition::{
    FeeStrategyDuplicateError, FeeStrategyEmptyError, FeeStrategyIndexOutOfBoundsError,
    FeeStrategyTooManyStepsError, InputBelowMinimumError, InputOutputBalanceMismatchError,
    InputWitnessCountMismatchError, OutputAddressAlsoInputError, OutputBelowMinimumError,
    TransitionNoInputsError, TransitionNoOutputsError, TransitionOverMaxInputsError,
    TransitionOverMaxOutputsError,
};
use crate::consensus::basic::BasicError;
use crate::state_transition::address_funds_transfer_transition::v0::AddressFundsTransferTransitionV0;
use crate::state_transition::StateTransitionStructureValidation;
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;
use std::collections::HashSet;

impl StateTransitionStructureValidation for AddressFundsTransferTransitionV0 {
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

        // Validate at least one output
        if self.outputs.is_empty() {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::TransitionNoOutputsError(TransitionNoOutputsError::new()).into(),
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

        // Validate fee strategy has no duplicates (efficiently using HashSet)
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

        // Validate each output is at least min_output_amount
        for amount in self.outputs.values() {
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

        // Validate inputs sum equals outputs sum (for transfers)
        let input_sum = self
            .inputs
            .values()
            .try_fold(0u64, |acc, (_, amount)| acc.checked_add(*amount));
        let input_sum = match input_sum {
            Some(sum) => sum,
            None => {
                return SimpleConsensusValidationResult::new_with_error(
                    BasicError::OverflowError(OverflowError::new("Input sum overflow".to_string()))
                        .into(),
                );
            }
        };

        let output_sum = self
            .outputs
            .values()
            .try_fold(0u64, |acc, amount| acc.checked_add(*amount));
        let output_sum = match output_sum {
            Some(sum) => sum,
            None => {
                return SimpleConsensusValidationResult::new_with_error(
                    BasicError::OverflowError(OverflowError::new(
                        "Output sum overflow".to_string(),
                    ))
                    .into(),
                );
            }
        };

        if input_sum != output_sum {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::InputOutputBalanceMismatchError(InputOutputBalanceMismatchError::new(
                    input_sum, output_sum,
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
    use crate::address_funds::{AddressWitness, PlatformAddress};
    use crate::consensus::ConsensusError;
    use assert_matches::assert_matches;
    use std::collections::BTreeMap;

    /// Helper: returns a valid funds transfer transition that passes all validations.
    /// input_sum == output_sum as required for transfers.
    fn valid_transfer_transition() -> AddressFundsTransferTransitionV0 {
        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([1u8; 20]), (0, 1_000_000));

        let mut outputs = BTreeMap::new();
        outputs.insert(PlatformAddress::P2pkh([2u8; 20]), 1_000_000);

        AddressFundsTransferTransitionV0 {
            inputs,
            outputs,
            fee_strategy: vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            user_fee_increase: 0,
            input_witnesses: vec![AddressWitness::P2pkh {
                signature: vec![0u8; 65].into(),
            }],
        }
    }

    #[test]
    fn should_validate_a_valid_transition() {
        let platform_version = PlatformVersion::latest();
        let transition = valid_transfer_transition();
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
        let mut transition = valid_transfer_transition();
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
    fn should_return_invalid_if_no_outputs() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_transfer_transition();
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
    fn should_return_invalid_if_too_many_inputs() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_transfer_transition();
        let max = platform_version.dpp.state_transitions.max_address_inputs;
        transition.inputs.clear();
        transition.input_witnesses.clear();
        let per_input = 1_000_000u64;
        for i in 0..=(max as usize) {
            let mut hash = [0u8; 20];
            hash[0] = (i & 0xFF) as u8;
            hash[1] = ((i >> 8) & 0xFF) as u8;
            transition
                .inputs
                .insert(PlatformAddress::P2pkh(hash), (0, per_input));
            transition.input_witnesses.push(AddressWitness::P2pkh {
                signature: vec![0u8; 65].into(),
            });
        }
        // Make outputs match
        let total = per_input * (max as u64 + 1);
        transition.outputs.clear();
        transition
            .outputs
            .insert(PlatformAddress::P2pkh([99u8; 20]), total);

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
        let mut transition = valid_transfer_transition();
        let max = platform_version.dpp.state_transitions.max_address_outputs;
        transition.outputs.clear();
        let per_output = 500_000u64;
        for i in 0..=(max as usize) {
            let mut hash = [0u8; 20];
            hash[0] = (i & 0xFF) as u8;
            hash[1] = ((i >> 8) & 0xFF) as u8;
            transition
                .outputs
                .insert(PlatformAddress::P2pkh(hash), per_output);
        }
        // Make inputs match
        let total = per_output * (max as u64 + 1);
        transition.inputs.clear();
        transition
            .inputs
            .insert(PlatformAddress::P2sh([99u8; 20]), (0, total));
        transition.input_witnesses = vec![AddressWitness::P2pkh {
            signature: vec![0u8; 65].into(),
        }];

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
        let mut transition = valid_transfer_transition();
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
        let shared_addr = PlatformAddress::P2pkh([1u8; 20]);
        let mut inputs = BTreeMap::new();
        inputs.insert(shared_addr, (0, 1_000_000));
        let mut outputs = BTreeMap::new();
        outputs.insert(shared_addr, 1_000_000);

        let transition = AddressFundsTransferTransitionV0 {
            inputs,
            outputs,
            fee_strategy: vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            user_fee_increase: 0,
            input_witnesses: vec![AddressWitness::P2pkh {
                signature: vec![0u8; 65].into(),
            }],
        };

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
        let mut transition = valid_transfer_transition();
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
        let mut transition = valid_transfer_transition();
        let max = platform_version
            .dpp
            .state_transitions
            .max_address_fee_strategies;
        transition.fee_strategy.clear();
        // Add enough inputs to support the indices
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
        // Fix output sum
        let total: u64 = 1_000_000 * (max as u64 + 1);
        transition.outputs.clear();
        transition
            .outputs
            .insert(PlatformAddress::P2sh([99u8; 20]), total);

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
        let mut transition = valid_transfer_transition();
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
        let mut transition = valid_transfer_transition();
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
    fn should_return_invalid_if_reduce_output_index_out_of_bounds() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_transfer_transition();
        transition.fee_strategy = vec![AddressFundsFeeStrategyStep::ReduceOutput(99)];

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
        let mut transition = valid_transfer_transition();
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
        let mut transition = valid_transfer_transition();
        transition.outputs.clear();
        transition
            .outputs
            .insert(PlatformAddress::P2pkh([2u8; 20]), 1);

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::OutputBelowMinimumError(_)
            )]
        );
    }

    #[test]
    fn should_return_invalid_if_input_sum_overflows() {
        let platform_version = PlatformVersion::latest();

        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([1u8; 20]), (0, u64::MAX));
        inputs.insert(PlatformAddress::P2pkh([3u8; 20]), (0, u64::MAX));

        let mut outputs = BTreeMap::new();
        outputs.insert(PlatformAddress::P2pkh([2u8; 20]), 1_000_000);

        let transition = AddressFundsTransferTransitionV0 {
            inputs,
            outputs,
            fee_strategy: vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            user_fee_increase: 0,
            input_witnesses: vec![
                AddressWitness::P2pkh {
                    signature: vec![0u8; 65].into(),
                },
                AddressWitness::P2pkh {
                    signature: vec![0u8; 65].into(),
                },
            ],
        };

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(BasicError::OverflowError(_))]
        );
    }

    #[test]
    fn should_return_invalid_if_output_sum_overflows() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_transfer_transition();
        transition.outputs.clear();
        transition
            .outputs
            .insert(PlatformAddress::P2pkh([2u8; 20]), u64::MAX);
        transition
            .outputs
            .insert(PlatformAddress::P2pkh([3u8; 20]), u64::MAX);

        let result = transition.validate_structure(platform_version);
        // Input sum check happens first but won't overflow with single input.
        // Output sum will overflow.
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(BasicError::OverflowError(_))]
        );
    }

    #[test]
    fn should_return_invalid_if_input_output_balance_mismatch() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_transfer_transition();
        // Input = 1_000_000 but output = 500_000
        transition.outputs.clear();
        transition
            .outputs
            .insert(PlatformAddress::P2pkh([2u8; 20]), 500_000);

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::InputOutputBalanceMismatchError(_)
            )]
        );
    }

    #[test]
    fn should_validate_multi_input_multi_output_transfer() {
        let platform_version = PlatformVersion::latest();

        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([1u8; 20]), (0, 1_000_000));
        inputs.insert(PlatformAddress::P2pkh([2u8; 20]), (0, 2_000_000));

        let mut outputs = BTreeMap::new();
        outputs.insert(PlatformAddress::P2pkh([3u8; 20]), 1_500_000);
        outputs.insert(PlatformAddress::P2pkh([4u8; 20]), 1_500_000);

        let transition = AddressFundsTransferTransitionV0 {
            inputs,
            outputs,
            fee_strategy: vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            user_fee_increase: 0,
            input_witnesses: vec![
                AddressWitness::P2pkh {
                    signature: vec![0u8; 65].into(),
                },
                AddressWitness::P2pkh {
                    signature: vec![0u8; 65].into(),
                },
            ],
        };

        let result = transition.validate_structure(platform_version);
        assert!(
            result.is_valid(),
            "Expected valid result, got errors: {:?}",
            result.errors
        );
    }
}
