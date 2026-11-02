use crate::consensus::basic::identity::InvalidIdentityCreditWithdrawalTransitionAmountError;
use crate::consensus::basic::state_transition::WithdrawalBelowMinAmountError;
use crate::consensus::basic::BasicError;
use crate::shielded::compute_shielded_withdrawal_fee;
use crate::state_transition::shielded_withdrawal_transition::v0::ShieldedWithdrawalTransitionV0;
use crate::validation::SimpleConsensusValidationResult;
use crate::withdrawal::{min_withdrawal_amount_with_core_fee, validate_core_fee_per_byte_cap};
use platform_version::version::PlatformVersion;

impl ShieldedWithdrawalTransitionV0 {
    /// Stateless rules from protocol version 14: the v0 rules plus a cap on `core_fee_per_byte`
    /// and a range check on the amount reserved for Core, which must leave
    /// `min_withdrawal_amount` above the Core fee. From v14 the Core fee is carved out of the
    /// withdrawn amount instead of being drawn from the Core credit pool on top of it.
    pub(super) fn validate_structure_v1(
        &self,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        let result = self.validate_structure_v0(platform_version);
        if !result.is_valid() {
            return result;
        }

        let result = validate_core_fee_per_byte_cap(self.core_fee_per_byte, platform_version);
        if !result.is_valid() {
            return result;
        }

        // The withdrawal document carries the unshielding amount net of the Platform fee. An
        // unshielding amount that does not even cover that fee is left to the shielded state
        // validation, which reports it as an insufficient fee rather than a short withdrawal.
        // The same state validation recomputes this fee and surfaces any error in computing
        // it, so a computation failure here also defers to it.
        let Ok(platform_fee) =
            compute_shielded_withdrawal_fee(self.actions.len(), platform_version)
        else {
            return SimpleConsensusValidationResult::new();
        };
        let Some(reserved_amount) = self.unshielding_amount.checked_sub(platform_fee) else {
            return SimpleConsensusValidationResult::new();
        };

        let minimum_reserved_amount =
            min_withdrawal_amount_with_core_fee(self.core_fee_per_byte, platform_version);
        let max_withdrawal_amount = platform_version.system_limits.max_withdrawal_amount;

        if reserved_amount < minimum_reserved_amount {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::WithdrawalBelowMinAmountError(WithdrawalBelowMinAmountError::new(
                    reserved_amount,
                    minimum_reserved_amount,
                    max_withdrawal_amount,
                ))
                .into(),
            );
        }

        if reserved_amount > max_withdrawal_amount {
            return SimpleConsensusValidationResult::new_with_error(
                InvalidIdentityCreditWithdrawalTransitionAmountError::new(
                    reserved_amount,
                    minimum_reserved_amount,
                    max_withdrawal_amount,
                )
                .into(),
            );
        }

        SimpleConsensusValidationResult::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::ConsensusError;
    use crate::identity::core_script::CoreScript;
    use crate::shielded::SerializedAction;
    use crate::state_transition::StateTransitionStructureValidation;
    use crate::withdrawal::{core_fee_in_credits, Pooling};
    use assert_matches::assert_matches;

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

    #[test]
    fn should_enforce_core_fee_limit_and_output_floor() {
        let platform_version = PlatformVersion::latest();
        let platform_fee =
            compute_shielded_withdrawal_fee(1, platform_version).expect("platform fee");
        let minimum_reserved_amount = platform_version.system_limits.min_withdrawal_amount
            + core_fee_in_credits(6_765).expect("Core fee");

        assert_matches!(
            transition(platform_fee + minimum_reserved_amount, 10_946)
                .validate_structure(platform_version)
                .errors
                .as_slice(),
            [ConsensusError::BasicError(
                BasicError::InvalidCreditWithdrawalTransitionCoreFeeError(_)
            )]
        );
        assert_matches!(
            transition(platform_fee + minimum_reserved_amount - 1, 6_765)
                .validate_structure(platform_version)
                .errors
                .as_slice(),
            [ConsensusError::BasicError(
                BasicError::WithdrawalBelowMinAmountError(_)
            )]
        );
        assert!(transition(platform_fee + minimum_reserved_amount, 6_765)
            .validate_structure(platform_version)
            .is_valid());
    }

    #[test]
    fn should_leave_an_unshielding_amount_below_the_platform_fee_to_state_validation() {
        let platform_version = PlatformVersion::latest();
        let platform_fee =
            compute_shielded_withdrawal_fee(1, platform_version).expect("platform fee");

        assert!(transition(platform_fee - 1, 1)
            .validate_structure(platform_version)
            .is_valid());
    }

    #[test]
    fn should_keep_the_v0_rules() {
        let platform_version = PlatformVersion::latest();
        let mut zero_anchor = transition(2_000_000_000, 1);
        zero_anchor.anchor = [0; 32];

        assert!(!zero_anchor.validate_structure(platform_version).is_valid());
    }
}
