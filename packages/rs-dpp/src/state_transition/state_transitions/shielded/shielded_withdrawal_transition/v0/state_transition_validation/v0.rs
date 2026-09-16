use crate::consensus::basic::identity::{
    InvalidCreditWithdrawalTransitionCoreFeeError,
    InvalidCreditWithdrawalTransitionOutputScriptError,
    NotImplementedCreditWithdrawalTransitionPoolingError,
};
use crate::consensus::basic::state_transition::ShieldedInvalidValueBalanceError;
use crate::consensus::basic::BasicError;
use crate::state_transition::identity_credit_withdrawal_transition::MIN_CORE_FEE_PER_BYTE;
use crate::state_transition::shielded_withdrawal_transition::v0::ShieldedWithdrawalTransitionV0;
use crate::state_transition::state_transitions::shielded::common_validation::{
    validate_actions_count, validate_anchor_not_zero, validate_encrypted_note_sizes,
    validate_proof_not_empty,
};
use crate::util::is_non_zero_fibonacci_number::is_non_zero_fibonacci_number;
use crate::validation::SimpleConsensusValidationResult;
use crate::withdrawal::Pooling;
use platform_version::version::PlatformVersion;

impl ShieldedWithdrawalTransitionV0 {
    /// Stateless rules as shipped in protocol version 13. Frozen: later protocol versions select
    /// `validate_structure_v1` and above through the dpp `validate_withdrawal_structure` version slot.
    pub(super) fn validate_structure_v0(
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

        // unshielding_amount must be positive and within i64::MAX
        if self.unshielding_amount == 0 {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::ShieldedInvalidValueBalanceError(
                    ShieldedInvalidValueBalanceError::new(
                        "shielded withdrawal unshielding_amount must be positive".to_string(),
                    ),
                )
                .into(),
            );
        }

        if self.unshielding_amount > i64::MAX as u64 {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::ShieldedInvalidValueBalanceError(
                    ShieldedInvalidValueBalanceError::new(
                        "shielded withdrawal unshielding_amount exceeds maximum allowed value"
                            .to_string(),
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

        // The shielded withdrawal carries the same transparent, Core-facing fields as
        // IdentityCreditWithdrawal (output_script, pooling, core_fee_per_byte), and the
        // transformer writes them straight into the queued withdrawal document that drives
        // the Core asset-unlock TxOut. Mirror the transparent path's invariants so a valid
        // Orchard proof cannot enqueue a non-standard/oversized output script (storage
        // griefing at the flat shielded fee), an unsupported pooling discriminant, or an
        // unrelayable/zero core_fee_per_byte.

        // Pooling is not yet supported: must be Never.
        if self.pooling != Pooling::Never {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::NotImplementedCreditWithdrawalTransitionPoolingError(
                    NotImplementedCreditWithdrawalTransitionPoolingError::new(self.pooling as u8),
                )
                .into(),
            );
        }

        // core_fee_per_byte must be a non-zero Fibonacci number.
        if !is_non_zero_fibonacci_number(self.core_fee_per_byte as u64) {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::InvalidCreditWithdrawalTransitionCoreFeeError(
                    InvalidCreditWithdrawalTransitionCoreFeeError::new(
                        self.core_fee_per_byte,
                        MIN_CORE_FEE_PER_BYTE,
                    ),
                )
                .into(),
            );
        }

        // output_script must be a canonical P2PKH or P2SH script.
        if !self.output_script.is_p2pkh() && !self.output_script.is_p2sh() {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::InvalidCreditWithdrawalTransitionOutputScriptError(
                    InvalidCreditWithdrawalTransitionOutputScriptError::new(
                        self.output_script.clone(),
                    ),
                )
                .into(),
            );
        }

        SimpleConsensusValidationResult::new()
    }
}
