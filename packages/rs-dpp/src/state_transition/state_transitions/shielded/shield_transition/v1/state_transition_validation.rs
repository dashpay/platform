use crate::address_funds::AddressFundsFeeStrategyStep;
use crate::consensus::basic::state_transition::{
    FeeStrategyDuplicateError, FeeStrategyEmptyError, FeeStrategyIndexOutOfBoundsError,
    FeeStrategyTooManyStepsError, InputBelowMinimumError, InputWitnessCountMismatchError,
    ShieldedInvalidValueBalanceError, TransitionNoInputsError,
};
use crate::consensus::basic::BasicError;
use crate::shielded::compute_shielded_verification_fee;
use crate::state_transition::shield_transition::v1::ShieldTransitionV1;
use crate::state_transition::state_transitions::shielded::common_validation::{
    validate_actions_count, validate_anchor_not_zero, validate_encrypted_note_sizes,
    validate_proof_not_empty,
};
use crate::state_transition::StateTransitionStructureValidation;
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;
use std::collections::HashSet;

impl StateTransitionStructureValidation for ShieldTransitionV1 {
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

        // Each action's encrypted_note must be exactly ENCRYPTED_NOTE_SIZE bytes
        let result = validate_encrypted_note_sizes(&self.actions);
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

        // Total input amounts must cover the shield amount PLUS the shielded COMPUTE fee
        // (`compute_shielded_verification_fee`: proof verification + per-action processing, NO storage
        // term). This is a stateless lower bound: the real per-action storage cost is metered by
        // GroveDB at execution and is unknowable here, so we deliberately do NOT add the 312-byte
        // storage ESTIMATE — doing so would falsely reject otherwise-valid transitions whose actual
        // metered storage is below the estimate.
        //
        // This is a cheap early reject. The AUTHORITATIVE funding gate is `validate_fees_of_event`
        // (drive-abci), which re-checks `metered_storage + metered_processing + compute_fee` against
        // the per-input balances AFTER the shield-amount reallocation. Anti-mint safety does NOT
        // depend on this `+fee` term: it depends on `Σrequested >= amount` enforced by the
        // reallocation plus that authoritative gate. `input_sum` here only bounds the sum of max
        // contributions.
        //
        // `compute_shielded_verification_fee` returns a `Result`, but this validator returns a
        // `SimpleConsensusValidationResult`, so we cannot `?`-propagate; we map an overflow to a
        // consensus error (reachable only via pathological fee constants).
        let minimum_fee =
            match compute_shielded_verification_fee(self.actions.len(), platform_version) {
                Ok(fee) => fee,
                Err(_) => {
                    return SimpleConsensusValidationResult::new_with_error(
                        BasicError::ShieldedInvalidValueBalanceError(
                            ShieldedInvalidValueBalanceError::new(
                                "minimum shielded fee computation overflowed".to_string(),
                            ),
                        )
                        .into(),
                    );
                }
            };
        let required_input = match self.amount.checked_add(minimum_fee) {
            Some(required) => required,
            None => {
                return SimpleConsensusValidationResult::new_with_error(
                    BasicError::ShieldedInvalidValueBalanceError(
                        ShieldedInvalidValueBalanceError::new(
                            "shield amount plus minimum shielded fee overflows".to_string(),
                        ),
                    )
                    .into(),
                );
            }
        };
        let input_sum = self
            .inputs
            .values()
            .try_fold(0u64, |acc, (_, amount)| acc.checked_add(*amount));
        match input_sum {
            Some(sum) if sum >= required_input => {}
            Some(sum) => {
                return SimpleConsensusValidationResult::new_with_error(
                    BasicError::ShieldedInvalidValueBalanceError(
                        ShieldedInvalidValueBalanceError::new(format!(
                            "total input amount ({}) is less than shield amount ({}) plus minimum shielded fee ({})",
                            sum, self.amount, minimum_fee
                        )),
                    )
                    .into(),
                );
            }
            None => {
                return SimpleConsensusValidationResult::new_with_error(
                    BasicError::ShieldedInvalidValueBalanceError(
                        ShieldedInvalidValueBalanceError::new(
                            "total input amounts overflow".to_string(),
                        ),
                    )
                    .into(),
                );
            }
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

            // Reject structurally-unusable fee-strategy steps here — cheaply, before Orchard proof
            // verification — rather than letting an out-of-range/no-op step slip through and only
            // surface later as a generic `AddressesNotEnoughFundsError`. A `Shield` has transparent
            // inputs but NO transparent outputs, so `DeductFromInput` must index a real input and
            // `ReduceOutput` can never apply. Mirrors `AddressFundingFromAssetLock`'s validator.
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
                    // A Shield has no transparent outputs (count 0), so any `ReduceOutput` is out of
                    // bounds.
                    return SimpleConsensusValidationResult::new_with_error(
                        BasicError::FeeStrategyIndexOutOfBoundsError(
                            FeeStrategyIndexOutOfBoundsError::new("ReduceOutput", *index, 0),
                        )
                        .into(),
                    );
                }
            }
        }

        SimpleConsensusValidationResult::new()
    }
}
