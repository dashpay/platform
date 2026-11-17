use platform_value::Identifier;
use crate::consensus::basic::BasicError;
use crate::consensus::basic::token::{InvalidTokenAmountError, InvalidTokenNoteTooBigError, TokenTransferToOurselfError};
use crate::consensus::ConsensusError;
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::MAX_DISTRIBUTION_PARAM;
use crate::ProtocolError;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use crate::state_transition::batch_transition::token_transfer_transition::v0::v0_methods::TokenTransferTransitionV0Methods;
use crate::state_transition::batch_transition::TokenTransferTransition;
use crate::tokens::MAX_TOKEN_NOTE_LEN;
use crate::validation::SimpleConsensusValidationResult;

pub(super) trait TokenTransferTransitionActionStructureValidationV0 {
    fn validate_structure_v0(
        &self,
        owner_id: Identifier,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}
impl TokenTransferTransitionActionStructureValidationV0 for TokenTransferTransition {
    fn validate_structure_v0(
        &self,
        owner_id: Identifier,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        if self.amount() > MAX_DISTRIBUTION_PARAM || self.amount() == 0 {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ConsensusError::BasicError(BasicError::InvalidTokenAmountError(
                    InvalidTokenAmountError::new(MAX_DISTRIBUTION_PARAM, self.amount()),
                )),
            ));
        }

        // Only a version 1 transfer carries a random minimum amount, and no protocol version
        // before 15 decodes one (`StateTransition::active_version_range`), so this check is
        // inert at every earlier version that selects this generation.
        if let Some(minimum) = self.random_minimum_amount() {
            if minimum == 0 || minimum >= self.amount() {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::BasicError(BasicError::InvalidTokenAmountError(
                        InvalidTokenAmountError::new(self.amount().saturating_sub(1), minimum),
                    )),
                ));
            }
        }

        if self.recipient_id() == owner_id {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ConsensusError::BasicError(BasicError::TokenTransferToOurselfError(
                    TokenTransferToOurselfError::new(self.base().token_id(), owner_id),
                )),
            ));
        }

        if let Some(public_note) = self.public_note() {
            if public_note.len() > MAX_TOKEN_NOTE_LEN {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::BasicError(BasicError::InvalidTokenNoteTooBigError(
                        InvalidTokenNoteTooBigError::new(
                            MAX_TOKEN_NOTE_LEN as u32,
                            "public_note",
                            public_note.len() as u32,
                        ),
                    )),
                ));
            }
        }

        if let Some(shared_encrypted_note) = self.shared_encrypted_note() {
            if shared_encrypted_note.2.len() > MAX_TOKEN_NOTE_LEN {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::BasicError(BasicError::InvalidTokenNoteTooBigError(
                        InvalidTokenNoteTooBigError::new(
                            MAX_TOKEN_NOTE_LEN as u32,
                            "shared_encrypted_note",
                            shared_encrypted_note.2.len() as u32,
                        ),
                    )),
                ));
            }
        }

        if let Some(private_encrypted_note) = self.private_encrypted_note() {
            if private_encrypted_note.2.len() > MAX_TOKEN_NOTE_LEN {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::BasicError(BasicError::InvalidTokenNoteTooBigError(
                        InvalidTokenNoteTooBigError::new(
                            MAX_TOKEN_NOTE_LEN as u32,
                            "private_encrypted_note",
                            private_encrypted_note.2.len() as u32,
                        ),
                    )),
                ));
            }
        }

        Ok(SimpleConsensusValidationResult::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
    use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
    use crate::state_transition::batch_transition::token_transfer_transition::{
        TokenTransferTransitionV0, TokenTransferTransitionV1,
    };

    fn make_base() -> TokenBaseTransition {
        TokenBaseTransition::V0(TokenBaseTransitionV0 {
            identity_contract_nonce: 1,
            token_contract_position: 0,
            data_contract_id: Identifier::default(),
            token_id: Identifier::new([1u8; 32]),
            using_group_info: None,
        })
    }

    fn make_valid_transfer(owner_id: Identifier) -> TokenTransferTransition {
        // Ensure recipient differs from owner
        let mut recipient_bytes = [2u8; 32];
        if Identifier::new(recipient_bytes) == owner_id {
            recipient_bytes = [3u8; 32];
        }
        TokenTransferTransition::V0(TokenTransferTransitionV0 {
            base: make_base(),
            amount: 1000,
            recipient_id: Identifier::new(recipient_bytes),
            public_note: None,
            shared_encrypted_note: None,
            private_encrypted_note: None,
        })
    }

    #[test]
    fn valid_transfer_passes() {
        let owner_id = Identifier::new([3u8; 32]);
        let transition = make_valid_transfer(owner_id);
        let result = transition.validate_structure_v0(owner_id).unwrap();
        assert!(result.is_valid(), "expected no errors: {:?}", result.errors);
    }

    #[test]
    fn zero_amount_returns_invalid_token_amount_error() {
        let owner_id = Identifier::new([3u8; 32]);
        let transition = TokenTransferTransition::V0(TokenTransferTransitionV0 {
            base: make_base(),
            amount: 0,
            recipient_id: Identifier::new([2u8; 32]),
            public_note: None,
            shared_encrypted_note: None,
            private_encrypted_note: None,
        });
        let result = transition.validate_structure_v0(owner_id).unwrap();
        assert_eq!(result.errors.len(), 1);
        assert!(matches!(
            &result.errors[0],
            ConsensusError::BasicError(BasicError::InvalidTokenAmountError(_))
        ));
    }

    #[test]
    fn amount_exceeding_max_returns_invalid_token_amount_error() {
        let owner_id = Identifier::new([3u8; 32]);
        let transition = TokenTransferTransition::V0(TokenTransferTransitionV0 {
            base: make_base(),
            amount: MAX_DISTRIBUTION_PARAM + 1,
            recipient_id: Identifier::new([2u8; 32]),
            public_note: None,
            shared_encrypted_note: None,
            private_encrypted_note: None,
        });
        let result = transition.validate_structure_v0(owner_id).unwrap();
        assert_eq!(result.errors.len(), 1);
        assert!(matches!(
            &result.errors[0],
            ConsensusError::BasicError(BasicError::InvalidTokenAmountError(_))
        ));
    }

    #[test]
    fn amount_at_max_distribution_param_passes() {
        let owner_id = Identifier::new([3u8; 32]);
        let transition = TokenTransferTransition::V0(TokenTransferTransitionV0 {
            base: make_base(),
            amount: MAX_DISTRIBUTION_PARAM,
            recipient_id: Identifier::new([2u8; 32]),
            public_note: None,
            shared_encrypted_note: None,
            private_encrypted_note: None,
        });
        let result = transition.validate_structure_v0(owner_id).unwrap();
        assert!(result.is_valid(), "expected no errors: {:?}", result.errors);
    }

    #[test]
    fn transfer_to_self_returns_token_transfer_to_ourself_error() {
        let owner_id = Identifier::new([5u8; 32]);
        let transition = TokenTransferTransition::V0(TokenTransferTransitionV0 {
            base: make_base(),
            amount: 100,
            recipient_id: owner_id,
            public_note: None,
            shared_encrypted_note: None,
            private_encrypted_note: None,
        });
        let result = transition.validate_structure_v0(owner_id).unwrap();
        assert_eq!(result.errors.len(), 1);
        assert!(matches!(
            &result.errors[0],
            ConsensusError::BasicError(BasicError::TokenTransferToOurselfError(_))
        ));
    }

    #[test]
    fn public_note_too_big_returns_error() {
        let owner_id = Identifier::new([3u8; 32]);
        let transition = TokenTransferTransition::V0(TokenTransferTransitionV0 {
            base: make_base(),
            amount: 100,
            recipient_id: Identifier::new([2u8; 32]),
            public_note: Some("x".repeat(MAX_TOKEN_NOTE_LEN + 1)),
            shared_encrypted_note: None,
            private_encrypted_note: None,
        });
        let result = transition.validate_structure_v0(owner_id).unwrap();
        assert_eq!(result.errors.len(), 1);
        assert!(matches!(
            &result.errors[0],
            ConsensusError::BasicError(BasicError::InvalidTokenNoteTooBigError(_))
        ));
    }

    #[test]
    fn public_note_at_max_length_passes() {
        let owner_id = Identifier::new([3u8; 32]);
        let transition = TokenTransferTransition::V0(TokenTransferTransitionV0 {
            base: make_base(),
            amount: 100,
            recipient_id: Identifier::new([2u8; 32]),
            public_note: Some("x".repeat(MAX_TOKEN_NOTE_LEN)),
            shared_encrypted_note: None,
            private_encrypted_note: None,
        });
        let result = transition.validate_structure_v0(owner_id).unwrap();
        assert!(result.is_valid(), "expected no errors: {:?}", result.errors);
    }

    #[test]
    fn shared_encrypted_note_too_big_returns_error() {
        let owner_id = Identifier::new([3u8; 32]);
        let transition = TokenTransferTransition::V0(TokenTransferTransitionV0 {
            base: make_base(),
            amount: 100,
            recipient_id: Identifier::new([2u8; 32]),
            public_note: None,
            shared_encrypted_note: Some((0, 0, vec![0u8; MAX_TOKEN_NOTE_LEN + 1])),
            private_encrypted_note: None,
        });
        let result = transition.validate_structure_v0(owner_id).unwrap();
        assert_eq!(result.errors.len(), 1);
        assert!(matches!(
            &result.errors[0],
            ConsensusError::BasicError(BasicError::InvalidTokenNoteTooBigError(_))
        ));
    }

    #[test]
    fn private_encrypted_note_too_big_returns_error() {
        let owner_id = Identifier::new([3u8; 32]);
        let transition = TokenTransferTransition::V0(TokenTransferTransitionV0 {
            base: make_base(),
            amount: 100,
            recipient_id: Identifier::new([2u8; 32]),
            public_note: None,
            shared_encrypted_note: None,
            private_encrypted_note: Some((0, 0, vec![0u8; MAX_TOKEN_NOTE_LEN + 1])),
        });
        let result = transition.validate_structure_v0(owner_id).unwrap();
        assert_eq!(result.errors.len(), 1);
        assert!(matches!(
            &result.errors[0],
            ConsensusError::BasicError(BasicError::InvalidTokenNoteTooBigError(_))
        ));
    }

    fn random_transfer(amount: u64, random_minimum_amount: Option<u64>) -> TokenTransferTransition {
        TokenTransferTransition::V1(TokenTransferTransitionV1 {
            base: make_base(),
            amount,
            recipient_id: Identifier::new([2u8; 32]),
            public_note: None,
            shared_encrypted_note: None,
            private_encrypted_note: None,
            random_minimum_amount,
        })
    }

    fn assert_invalid_random_minimum(result: SimpleConsensusValidationResult, minimum: u64) {
        assert_eq!(result.errors.len(), 1);
        assert!(matches!(
            &result.errors[0],
            ConsensusError::BasicError(BasicError::InvalidTokenAmountError(e))
                if e.token_amount() == minimum
        ));
    }

    #[test]
    fn should_accept_a_random_amount_from_one_to_below_the_amount() {
        let owner_id = Identifier::new([3u8; 32]);
        for minimum in [1, 500, 999] {
            let result = random_transfer(1000, Some(minimum))
                .validate_structure_v0(owner_id)
                .unwrap();
            assert!(result.is_valid(), "{minimum}: {:?}", result.errors);
        }
        let result = random_transfer(1000, None)
            .validate_structure_v0(owner_id)
            .unwrap();
        assert!(result.is_valid(), "expected no errors: {:?}", result.errors);
    }

    #[test]
    fn should_refuse_a_random_minimum_amount_of_zero() {
        let owner_id = Identifier::new([3u8; 32]);
        let result = random_transfer(1000, Some(0))
            .validate_structure_v0(owner_id)
            .unwrap();
        assert_invalid_random_minimum(result, 0);
    }

    #[test]
    fn should_refuse_a_random_minimum_amount_not_below_the_amount() {
        let owner_id = Identifier::new([3u8; 32]);
        for minimum in [1000, 1001] {
            let result = random_transfer(1000, Some(minimum))
                .validate_structure_v0(owner_id)
                .unwrap();
            assert_invalid_random_minimum(result, minimum);
        }
        // An amount of 1 leaves no minimum to draw from
        let result = random_transfer(1, Some(1))
            .validate_structure_v0(owner_id)
            .unwrap();
        assert_invalid_random_minimum(result, 1);
    }
}
