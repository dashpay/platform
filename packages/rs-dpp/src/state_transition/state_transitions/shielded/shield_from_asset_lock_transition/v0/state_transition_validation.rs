use crate::consensus::basic::state_transition::ShieldedInvalidValueBalanceError;
use crate::consensus::basic::BasicError;
use crate::state_transition::shield_from_asset_lock_transition::v0::ShieldFromAssetLockTransitionV0;
use crate::state_transition::state_transitions::shielded::common_validation::{
    validate_actions_count, validate_anchor_not_zero, validate_proof_not_empty,
};
use crate::state_transition::StateTransitionStructureValidation;
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;

impl StateTransitionStructureValidation for ShieldFromAssetLockTransitionV0 {
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

        // value_balance must be > 0 (credits flowing into pool)
        if self.value_balance == 0 {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::ShieldedInvalidValueBalanceError(
                    ShieldedInvalidValueBalanceError::new(
                        "shield_from_asset_lock value_balance must be greater than 0".to_string(),
                    ),
                )
                .into(),
            );
        }

        // value_balance must fit in i64 (Orchard protocol uses i64 internally)
        if self.value_balance > i64::MAX as u64 {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::ShieldedInvalidValueBalanceError(
                    ShieldedInvalidValueBalanceError::new(
                        "shield_from_asset_lock value_balance exceeds i64::MAX".to_string(),
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

        SimpleConsensusValidationResult::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::ConsensusError;
    use crate::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
    use crate::identity::state_transition::asset_lock_proof::AssetLockProof;
    use assert_matches::assert_matches;
    use dashcore::OutPoint;
    use platform_value::BinaryData;

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

    fn valid_shield_from_asset_lock_transition() -> ShieldFromAssetLockTransitionV0 {
        let chain_proof = ChainAssetLockProof {
            core_chain_locked_height: 100,
            out_point: OutPoint::from([11u8; 36]),
        };

        ShieldFromAssetLockTransitionV0 {
            asset_lock_proof: AssetLockProof::Chain(chain_proof),
            actions: vec![dummy_action()],
            value_balance: 1_000_000u64,
            anchor: [7u8; 32],
            proof: vec![8u8; 100],
            binding_signature: [9u8; 64],
            signature: BinaryData::new(vec![10u8; 65]),
        }
    }

    #[test]
    fn should_validate_a_valid_transition() {
        let platform_version = PlatformVersion::latest();
        let transition = valid_shield_from_asset_lock_transition();
        let result = transition.validate_structure(platform_version);
        assert!(
            result.is_valid(),
            "Expected valid result, got errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn should_reject_empty_actions() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shield_from_asset_lock_transition();
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
        let mut transition = valid_shield_from_asset_lock_transition();
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
    fn should_reject_zero_value_balance() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shield_from_asset_lock_transition();
        transition.value_balance = 0;

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedInvalidValueBalanceError(_)
            )]
        );
    }

    #[test]
    fn should_reject_value_balance_exceeding_i64_max() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shield_from_asset_lock_transition();
        transition.value_balance = i64::MAX as u64 + 1;

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedInvalidValueBalanceError(_)
            )]
        );
    }

    #[test]
    fn should_accept_value_balance_at_i64_max() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shield_from_asset_lock_transition();
        transition.value_balance = i64::MAX as u64;

        let result = transition.validate_structure(platform_version);
        assert!(
            result.is_valid(),
            "Expected valid result, got errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn should_reject_empty_proof() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shield_from_asset_lock_transition();
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
        let mut transition = valid_shield_from_asset_lock_transition();
        transition.anchor = [0u8; 32];

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedZeroAnchorError(_)
            )]
        );
    }
}
