use crate::consensus::basic::state_transition::{
    FeeStrategyDuplicateError, FeeStrategyEmptyError, FeeStrategyTooManyStepsError,
    InputBelowMinimumError, InputWitnessCountMismatchError, ShieldedInvalidValueBalanceError,
    TransitionNoInputsError,
};
use crate::consensus::basic::BasicError;
use crate::state_transition::shield_transition::v0::ShieldTransitionV0;
use crate::state_transition::state_transitions::shielded::common_validation::{
    validate_actions_count, validate_anchor_not_zero, validate_proof_not_empty,
};
use crate::state_transition::StateTransitionStructureValidation;
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;
use std::collections::HashSet;

impl StateTransitionStructureValidation for ShieldTransitionV0 {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        // Actions count must be in [1, max]
        let result = validate_actions_count(
            &self.actions,
            platform_version
                .system_limits
                .max_shielded_transition_actions,
        );
        if !result.is_valid() {
            return result;
        }

        // Inputs must not be empty (shield requires address funding)
        if self.inputs.is_empty() {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::TransitionNoInputsError(TransitionNoInputsError::new()).into(),
            );
        }

        // Input witnesses must match inputs count
        if self.inputs.len() != self.input_witnesses.len() {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::InputWitnessCountMismatchError(InputWitnessCountMismatchError::new(
                    self.inputs.len().min(u16::MAX as usize) as u16,
                    self.input_witnesses.len().min(u16::MAX as usize) as u16,
                ))
                .into(),
            );
        }

        // Validate each input amount is > 0
        let min_input_amount = platform_version
            .dpp
            .state_transitions
            .address_funds
            .min_input_amount;
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

        // amount must be positive (credits flowing into pool)
        if self.amount == 0 {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::ShieldedInvalidValueBalanceError(
                    ShieldedInvalidValueBalanceError::new(
                        "shield amount must be greater than zero".to_string(),
                    ),
                )
                .into(),
            );
        }

        // amount must fit in i64 (Orchard protocol uses i64 internally for value_balance)
        if self.amount > i64::MAX as u64 {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::ShieldedInvalidValueBalanceError(
                    ShieldedInvalidValueBalanceError::new(
                        "shield amount exceeds maximum allowed value".to_string(),
                    ),
                )
                .into(),
            );
        }

        // Proof must not be empty
        let result = validate_proof_not_empty(&self.proof);
        if !result.is_valid() {
            return result;
        }

        // Anchor must not be all zeros
        let result = validate_anchor_not_zero(&self.anchor);
        if !result.is_valid() {
            return result;
        }

        // Fee strategy validation (reuse address funds patterns)
        if self.fee_strategy.is_empty() {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::FeeStrategyEmptyError(FeeStrategyEmptyError::new()).into(),
            );
        }

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

        let mut seen = HashSet::with_capacity(self.fee_strategy.len());
        for step in &self.fee_strategy {
            if !seen.insert(step) {
                return SimpleConsensusValidationResult::new_with_error(
                    BasicError::FeeStrategyDuplicateError(FeeStrategyDuplicateError::new()).into(),
                );
            }
        }

        SimpleConsensusValidationResult::new()
    }
}
