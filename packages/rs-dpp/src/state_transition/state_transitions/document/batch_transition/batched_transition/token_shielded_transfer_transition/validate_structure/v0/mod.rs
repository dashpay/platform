use crate::state_transition::batch_transition::token_shielded_transfer_transition::v0::v0_methods::TokenShieldedTransferTransitionV0Methods;
use crate::state_transition::batch_transition::TokenShieldedTransferTransition;
use crate::state_transition::state_transitions::shielded::common_validation::{
    validate_actions_count, validate_anchor_not_zero, validate_encrypted_note_sizes,
    validate_proof_not_empty,
};
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

pub(super) trait TokenShieldedTransferTransitionActionStructureValidationV0 {
    fn validate_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl TokenShieldedTransferTransitionActionStructureValidationV0
    for TokenShieldedTransferTransition
{
    /// The same stateless bundle checks the credit-pool transitions run (action count, note
    /// sizes, non-empty proof, non-zero anchor). There is no amount to bound: the value
    /// balance is fixed at zero by the state validator.
    fn validate_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let result = validate_actions_count(
            self.actions(),
            platform_version
                .system_limits
                .max_shielded_transition_actions,
        );
        if !result.is_valid() {
            return Ok(result);
        }

        let result = validate_encrypted_note_sizes(self.actions());
        if !result.is_valid() {
            return Ok(result);
        }

        let result = validate_proof_not_empty(self.proof());
        if !result.is_valid() {
            return Ok(result);
        }

        Ok(validate_anchor_not_zero(self.anchor()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::basic::BasicError;
    use crate::consensus::ConsensusError;
    use crate::shielded::SerializedAction;
    use crate::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
    use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
    use crate::state_transition::batch_transition::token_shielded_transfer_transition::v0::TokenShieldedTransferTransitionV0;
    use platform_value::Identifier;

    fn action() -> SerializedAction {
        SerializedAction {
            nullifier: [1u8; 32],
            rk: [2u8; 32],
            cmx: [3u8; 32],
            encrypted_note: vec![4u8; 216],
            cv_net: [5u8; 32],
            spend_auth_sig: [6u8; 64],
        }
    }

    fn make_transition(actions: usize) -> TokenShieldedTransferTransition {
        TokenShieldedTransferTransition::V0(TokenShieldedTransferTransitionV0 {
            base: TokenBaseTransition::V0(TokenBaseTransitionV0 {
                identity_contract_nonce: 1,
                token_contract_position: 0,
                data_contract_id: Identifier::default(),
                token_id: Identifier::default(),
                using_group_info: None,
            }),
            actions: vec![action(); actions],
            anchor: [9u8; 32],
            proof: vec![1u8; 10],
            binding_signature: [7u8; 64],
        })
    }

    #[test]
    fn valid_transfer_passes() {
        let result = make_transition(2)
            .validate_structure_v0(PlatformVersion::latest())
            .unwrap();
        assert!(result.is_valid(), "{:?}", result.errors);
    }

    #[test]
    fn empty_actions_are_rejected() {
        let result = make_transition(0)
            .validate_structure_v0(PlatformVersion::latest())
            .unwrap();
        assert!(matches!(
            result.errors.first(),
            Some(ConsensusError::BasicError(
                BasicError::ShieldedNoActionsError(_)
            ))
        ));
    }

    #[test]
    fn too_many_actions_are_rejected() {
        let max = PlatformVersion::latest()
            .system_limits
            .max_shielded_transition_actions as usize;
        let result = make_transition(max + 1)
            .validate_structure_v0(PlatformVersion::latest())
            .unwrap();
        assert!(matches!(
            result.errors.first(),
            Some(ConsensusError::BasicError(
                BasicError::ShieldedTooManyActionsError(_)
            ))
        ));
    }

    #[test]
    fn zero_anchor_is_rejected() {
        let mut transition = make_transition(1);
        let TokenShieldedTransferTransition::V0(v0) = &mut transition;
        v0.anchor = [0u8; 32];
        let result = transition
            .validate_structure_v0(PlatformVersion::latest())
            .unwrap();
        assert!(matches!(
            result.errors.first(),
            Some(ConsensusError::BasicError(
                BasicError::ShieldedZeroAnchorError(_)
            ))
        ));
    }
}
