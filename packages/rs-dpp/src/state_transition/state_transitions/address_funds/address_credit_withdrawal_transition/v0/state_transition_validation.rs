use crate::address_funds::AddressFundsFeeWithWithdrawalStrategyStep;
use crate::consensus::basic::state_transition::{
    FeeStrategyDuplicateError, FeeStrategyEmptyError, FeeStrategyIndexOutOfBoundsError,
    FeeStrategyReduceWithdrawalNotLastError, FeeStrategyTooManyStepsError, InputBelowMinimumError,
    InputWitnessCountMismatchError, OutputBelowMinimumError, TransitionNoInputsError,
    TransitionOverMaxInputsError,
};
use crate::consensus::basic::BasicError;
use crate::state_transition::address_credit_withdrawal_transition::v0::AddressCreditWithdrawalTransitionV0;
use crate::state_transition::StateTransitionStructureValidation;
use crate::validation::SimpleConsensusValidationResult;
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

        // Validate ReduceWithdrawal is the last step if present
        for (i, step) in self.fee_strategy.iter().enumerate() {
            if matches!(
                step,
                AddressFundsFeeWithWithdrawalStrategyStep::ReduceWithdrawal
            ) && i != self.fee_strategy.len() - 1
            {
                return SimpleConsensusValidationResult::new_with_error(
                    BasicError::FeeStrategyReduceWithdrawalNotLastError(
                        FeeStrategyReduceWithdrawalNotLastError::new(),
                    )
                    .into(),
                );
            }
        }

        // Calculate number of outputs (0 or 1 for optional output)
        let output_count = if self.output.is_some() { 1 } else { 0 };

        // Validate fee strategy indices are within bounds
        for step in &self.fee_strategy {
            match step {
                AddressFundsFeeWithWithdrawalStrategyStep::DeductFromInput(index) => {
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
                AddressFundsFeeWithWithdrawalStrategyStep::ReduceOutput(index) => {
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
                AddressFundsFeeWithWithdrawalStrategyStep::ReduceWithdrawal => {
                    // ReduceWithdrawal doesn't have an index, so no bounds checking needed
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

        // Note: The withdrawal amount is implicitly input_sum - output_sum
        // No explicit balance check needed here as the withdrawal amount is computed, not specified

        SimpleConsensusValidationResult::new()
    }
}
