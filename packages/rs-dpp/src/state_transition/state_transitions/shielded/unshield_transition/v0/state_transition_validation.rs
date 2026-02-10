use crate::consensus::basic::overflow_error::OverflowError;
use crate::consensus::basic::BasicError;
use crate::state_transition::unshield_transition::v0::UnshieldTransitionV0;
use crate::state_transition::StateTransitionStructureValidation;
use crate::validation::SimpleConsensusValidationResult;
use platform_version::version::PlatformVersion;

impl StateTransitionStructureValidation for UnshieldTransitionV0 {
    fn validate_structure(
        &self,
        _platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        // Actions must not be empty
        if self.actions.is_empty() {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::OverflowError(OverflowError::new(
                    "Unshield transition must have at least one action".to_string(),
                ))
                .into(),
            );
        }

        // Amount must be > 0
        if self.amount == 0 {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::OverflowError(OverflowError::new(
                    "Unshield transition amount must be greater than zero".to_string(),
                ))
                .into(),
            );
        }

        // value_balance must be positive (credits flowing out of pool = amount + fee)
        if self.value_balance <= 0 {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::OverflowError(OverflowError::new(
                    "Unshield transition value_balance must be positive".to_string(),
                ))
                .into(),
            );
        }

        // value_balance must be >= amount (value_balance = amount + fee)
        if (self.value_balance as u64) < self.amount {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::OverflowError(OverflowError::new(
                    "Unshield transition value_balance must be >= amount".to_string(),
                ))
                .into(),
            );
        }

        // Proof must not be empty
        if self.proof.is_empty() {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::OverflowError(OverflowError::new(
                    "Unshield transition proof must not be empty".to_string(),
                ))
                .into(),
            );
        }

        // Binding signature must be exactly 64 bytes
        if self.binding_signature.len() != 64 {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::OverflowError(OverflowError::new(
                    "Unshield transition binding_signature must be exactly 64 bytes".to_string(),
                ))
                .into(),
            );
        }

        // Each action's spend_auth_sig must be exactly 64 bytes
        for action in &self.actions {
            if action.spend_auth_sig.len() != 64 {
                return SimpleConsensusValidationResult::new_with_error(
                    BasicError::OverflowError(OverflowError::new(
                        "Each action spend_auth_sig must be exactly 64 bytes".to_string(),
                    ))
                    .into(),
                );
            }
        }

        // Anchor must not be all zeros
        if self.anchor == [0u8; 32] {
            return SimpleConsensusValidationResult::new_with_error(
                BasicError::OverflowError(OverflowError::new(
                    "Unshield transition anchor must not be all zeros".to_string(),
                ))
                .into(),
            );
        }

        SimpleConsensusValidationResult::new()
    }
}
