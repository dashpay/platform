use crate::consensus::basic::state_transition::{
    FeeStrategyDuplicateError, FeeStrategyEmptyError, FeeStrategyTooManyStepsError,
    InputBelowMinimumError, InputWitnessCountMismatchError, ShieldedInvalidValueBalanceError,
    TransitionNoInputsError,
};
use crate::consensus::basic::BasicError;
use crate::state_transition::shield_transition::v0::ShieldTransitionV0;
use crate::state_transition::state_transitions::shielded::common_validation::{
    validate_actions_count, validate_anchor_not_zero, validate_encrypted_note_sizes,
    validate_proof_not_empty,
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

        // Total input amounts must cover the shield amount.
        // Without this check, an attacker could provide small inputs but a large
        // shield amount, crediting the pool more than the inputs debited.
        let input_sum = self
            .inputs
            .values()
            .try_fold(0u64, |acc, (_, amount)| acc.checked_add(*amount));
        match input_sum {
            Some(sum) if sum >= self.amount => {}
            Some(sum) => {
                return SimpleConsensusValidationResult::new_with_error(
                    BasicError::ShieldedInvalidValueBalanceError(
                        ShieldedInvalidValueBalanceError::new(format!(
                            "total input amount ({}) is less than shield amount ({})",
                            sum, self.amount
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
        }

        SimpleConsensusValidationResult::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address_funds::{AddressFundsFeeStrategyStep, AddressWitness, PlatformAddress};
    use crate::consensus::ConsensusError;
    use assert_matches::assert_matches;
    use std::collections::BTreeMap;

    fn dummy_action() -> crate::shielded::SerializedAction {
        crate::shielded::SerializedAction {
            nullifier: [1u8; 32],
            rk: [2u8; 32],
            cmx: [3u8; 32],
            encrypted_note: vec![4u8; 216],
            cv_net: [5u8; 32],
            spend_auth_sig: [6u8; 64],
        }
    }

    /// Creates a valid ShieldTransitionV0 that passes all validation checks.
    fn valid_shield_transition() -> ShieldTransitionV0 {
        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([1u8; 20]), (0u32, 1_000_000u64));

        ShieldTransitionV0 {
            inputs,
            actions: vec![dummy_action()],
            amount: 500_000u64,
            anchor: [7u8; 32],
            proof: vec![8u8; 100],
            binding_signature: [9u8; 64],
            fee_strategy: vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
            user_fee_increase: 0u16,
            input_witnesses: vec![AddressWitness::P2pkh {
                signature: vec![0u8; 65].into(),
            }],
        }
    }

    #[test]
    fn should_validate_a_valid_shield_transition() {
        let platform_version = PlatformVersion::latest();
        let transition = valid_shield_transition();
        let result = transition.validate_structure(platform_version);
        assert!(
            result.is_valid(),
            "Expected valid result, got errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn should_reject_invalid_encrypted_note_size() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shield_transition();
        transition.actions[0].encrypted_note = vec![4u8; 100]; // Wrong size

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedEncryptedNoteSizeMismatchError(_)
            )]
        );
    }

    #[test]
    fn should_reject_empty_actions() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shield_transition();
        transition.actions.clear();

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedNoActionsError(_)
            )]
        );
    }

    #[test]
    fn should_reject_too_many_actions() {
        let platform_version = PlatformVersion::latest();
        let max = platform_version
            .system_limits
            .max_shielded_transition_actions;
        let mut transition = valid_shield_transition();
        transition.actions = vec![dummy_action(); max as usize + 1];

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedTooManyActionsError(_)
            )]
        );
    }

    #[test]
    fn should_reject_empty_inputs() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shield_transition();
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
    fn should_reject_witness_count_mismatch() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shield_transition();
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
    fn should_reject_input_below_minimum() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shield_transition();
        // Set the input amount to 1 (below minimum of 100_000)
        transition.inputs.clear();
        transition
            .inputs
            .insert(PlatformAddress::P2pkh([1u8; 20]), (0u32, 1u64));

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::InputBelowMinimumError(_)
            )]
        );
    }

    #[test]
    fn should_reject_zero_amount() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shield_transition();
        transition.amount = 0;

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedInvalidValueBalanceError(_)
            )]
        );
    }

    #[test]
    fn should_reject_amount_exceeding_i64_max() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shield_transition();
        transition.amount = i64::MAX as u64 + 1;
        // Also make input sum large enough
        transition.inputs.clear();
        transition
            .inputs
            .insert(PlatformAddress::P2pkh([1u8; 20]), (0u32, u64::MAX));

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedInvalidValueBalanceError(_)
            )]
        );
    }

    #[test]
    fn should_reject_input_sum_less_than_shield_amount() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shield_transition();
        // Input = 1_000_000 but amount = 2_000_000
        transition.amount = 2_000_000;

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedInvalidValueBalanceError(_)
            )]
        );
    }

    #[test]
    fn should_reject_input_sum_overflow() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shield_transition();
        transition.inputs.clear();
        transition
            .inputs
            .insert(PlatformAddress::P2pkh([1u8; 20]), (0u32, u64::MAX));
        transition
            .inputs
            .insert(PlatformAddress::P2pkh([2u8; 20]), (0u32, u64::MAX));
        transition.input_witnesses = vec![
            AddressWitness::P2pkh {
                signature: vec![0u8; 65].into(),
            },
            AddressWitness::P2pkh {
                signature: vec![0u8; 65].into(),
            },
        ];
        transition.amount = 1_000_000;

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedInvalidValueBalanceError(_)
            )]
        );
    }

    #[test]
    fn should_reject_empty_proof() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shield_transition();
        transition.proof.clear();

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedEmptyProofError(_)
            )]
        );
    }

    #[test]
    fn should_reject_zero_anchor() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shield_transition();
        transition.anchor = [0u8; 32];

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedZeroAnchorError(_)
            )]
        );
    }

    #[test]
    fn should_reject_empty_fee_strategy() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shield_transition();
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
    fn should_reject_too_many_fee_strategy_steps() {
        let platform_version = PlatformVersion::latest();
        let max = platform_version
            .dpp
            .state_transitions
            .max_address_fee_strategies;
        let mut transition = valid_shield_transition();
        // Build enough inputs to support the indices
        transition.inputs.clear();
        transition.input_witnesses.clear();
        transition.fee_strategy.clear();
        for i in 0..=(max as usize) {
            let mut hash = [0u8; 20];
            hash[0] = (i & 0xFF) as u8;
            hash[1] = ((i >> 8) & 0xFF) as u8;
            transition
                .inputs
                .insert(PlatformAddress::P2pkh(hash), (0u32, 1_000_000u64));
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
    fn should_reject_duplicate_fee_strategy_steps() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shield_transition();
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
    fn should_accept_input_sum_exactly_equal_to_amount() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shield_transition();
        // Set input exactly equal to amount
        transition.inputs.clear();
        transition
            .inputs
            .insert(PlatformAddress::P2pkh([1u8; 20]), (0u32, 500_000u64));
        transition.amount = 500_000;

        let result = transition.validate_structure(platform_version);
        assert!(
            result.is_valid(),
            "Expected valid result, got errors: {:?}",
            result.errors
        );
    }
}
