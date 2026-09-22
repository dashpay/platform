mod v0;
mod v1;

use crate::consensus::basic::unsupported_version_error::UnsupportedVersionError;
use crate::consensus::basic::BasicError;
use crate::state_transition::shielded_withdrawal_transition::v0::ShieldedWithdrawalTransitionV0;
use crate::state_transition::StateTransitionStructureValidation;
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;

impl StateTransitionStructureValidation for ShieldedWithdrawalTransitionV0 {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        match platform_version
            .dpp
            .state_transitions
            .shielded
            .validate_withdrawal_structure
        {
            0 => self.validate_structure_v0(platform_version),
            1 => self.validate_structure_v1(platform_version),
            version => SimpleConsensusValidationResult::new_with_error(
                BasicError::UnsupportedVersionError(UnsupportedVersionError::new(version, 0, 1))
                    .into(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::ConsensusError;
    use crate::identity::core_script::CoreScript;
    use crate::shielded::{compute_shielded_withdrawal_fee, SerializedAction};
    use crate::withdrawal::{core_fee_in_credits, Pooling};
    use assert_matches::assert_matches;

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

    fn valid_shielded_withdrawal_transition() -> ShieldedWithdrawalTransitionV0 {
        ShieldedWithdrawalTransitionV0 {
            actions: vec![dummy_action()],
            unshielding_amount: 1_000_000u64,
            anchor: [7u8; 32],
            proof: vec![8u8; 100],
            binding_signature: [9u8; 64],
            core_fee_per_byte: 1u32,
            pooling: Pooling::Never,
            output_script: CoreScript::new_p2pkh([11u8; 20]),
        }
    }

    #[test]
    fn should_validate_a_valid_transition() {
        let platform_version = PlatformVersion::latest();
        let transition = valid_shielded_withdrawal_transition();
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
        let mut transition = valid_shielded_withdrawal_transition();
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
        let mut transition = valid_shielded_withdrawal_transition();
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
        let mut transition = valid_shielded_withdrawal_transition();
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
    fn should_reject_zero_unshielding_amount() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shielded_withdrawal_transition();
        transition.unshielding_amount = 0;

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedInvalidValueBalanceError(_)
            )]
        );
    }

    #[test]
    fn should_reject_unshielding_amount_exceeding_i64_max() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shielded_withdrawal_transition();
        transition.unshielding_amount = i64::MAX as u64 + 1;

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::ShieldedInvalidValueBalanceError(_)
            )]
        );
    }

    #[test]
    fn should_pass_the_value_balance_bound_at_i64_max() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shielded_withdrawal_transition();
        transition.unshielding_amount = i64::MAX as u64;

        // The value-balance bound admits i64::MAX; what rejects it from v14 is the withdrawal
        // maximum on the amount reserved for Core.
        assert_matches!(
            transition
                .validate_structure(platform_version)
                .errors
                .as_slice(),
            [ConsensusError::BasicError(
                BasicError::InvalidIdentityCreditWithdrawalTransitionAmountError(_)
            )]
        );
    }

    #[test]
    fn should_reject_empty_proof() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shielded_withdrawal_transition();
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
        let mut transition = valid_shielded_withdrawal_transition();
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
    fn should_reject_non_never_pooling() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shielded_withdrawal_transition();
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
    fn should_reject_zero_core_fee_per_byte() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shielded_withdrawal_transition();
        transition.core_fee_per_byte = 0;

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::InvalidCreditWithdrawalTransitionCoreFeeError(_)
            )]
        );
    }

    #[test]
    fn should_reject_non_fibonacci_core_fee_per_byte() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shielded_withdrawal_transition();
        transition.core_fee_per_byte = 4; // 4 is not a Fibonacci number

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::InvalidCreditWithdrawalTransitionCoreFeeError(_)
            )]
        );
    }

    #[test]
    fn should_reject_non_standard_output_script() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shielded_withdrawal_transition();
        // OP_RETURN-style script: neither P2PKH nor P2SH.
        transition.output_script = CoreScript::from_bytes(vec![0x6a, 0x01, 0x02]);

        let result = transition.validate_structure(platform_version);
        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::InvalidCreditWithdrawalTransitionOutputScriptError(_)
            )]
        );
    }

    #[test]
    fn should_accept_p2sh_output_script() {
        let platform_version = PlatformVersion::latest();
        let mut transition = valid_shielded_withdrawal_transition();
        transition.output_script = CoreScript::new_p2sh([12u8; 20]);

        let result = transition.validate_structure(platform_version);
        assert!(
            result.is_valid(),
            "Expected valid result, got errors: {:?}",
            result.errors
        );
    }

    fn transition(
        unshielding_amount: u64,
        core_fee_per_byte: u32,
    ) -> ShieldedWithdrawalTransitionV0 {
        ShieldedWithdrawalTransitionV0 {
            actions: vec![SerializedAction {
                nullifier: [1; 32],
                rk: [2; 32],
                cmx: [3; 32],
                encrypted_note: vec![4; 216],
                cv_net: [5; 32],
                spend_auth_sig: [6; 64],
            }],
            unshielding_amount,
            anchor: [7; 32],
            proof: vec![8; 100],
            binding_signature: [9; 64],
            core_fee_per_byte,
            pooling: Pooling::Never,
            output_script: CoreScript::new_p2pkh([11; 20]),
        }
    }

    fn at_the_old_minimum(platform_version: &PlatformVersion) -> ShieldedWithdrawalTransitionV0 {
        let platform_fee =
            compute_shielded_withdrawal_fee(1, platform_version).expect("platform fee");
        transition(
            platform_fee + platform_version.system_limits.min_withdrawal_amount,
            1,
        )
    }

    #[test]
    fn protocol_version_14_adds_the_core_fee_to_the_minimum() {
        let version_13 = PlatformVersion::get(13).expect("protocol version 13");
        let version_14 = PlatformVersion::get(14).expect("protocol version 14");

        assert!(at_the_old_minimum(version_13)
            .validate_structure(version_13)
            .is_valid());
        assert_matches!(
            at_the_old_minimum(version_14)
                .validate_structure(version_14)
                .errors
                .as_slice(),
            [ConsensusError::BasicError(
                BasicError::WithdrawalBelowMinAmountError(_)
            )]
        );

        let platform_fee = compute_shielded_withdrawal_fee(1, version_14).expect("platform fee");
        let above_the_fee = transition(
            platform_fee
                + version_14.system_limits.min_withdrawal_amount
                + core_fee_in_credits(1).expect("core fee"),
            1,
        );
        assert!(above_the_fee.validate_structure(version_14).is_valid());
    }

    #[test]
    fn protocol_version_14_caps_the_core_fee_rate() {
        let version_13 = PlatformVersion::get(13).expect("protocol version 13");
        let version_14 = PlatformVersion::get(14).expect("protocol version 14");
        let over_the_cap = transition(3_000_000_000, 10_946);

        assert!(over_the_cap.validate_structure(version_13).is_valid());
        assert_matches!(
            over_the_cap
                .validate_structure(version_14)
                .errors
                .as_slice(),
            [ConsensusError::BasicError(
                BasicError::InvalidCreditWithdrawalTransitionCoreFeeError(_)
            )]
        );
    }
}
