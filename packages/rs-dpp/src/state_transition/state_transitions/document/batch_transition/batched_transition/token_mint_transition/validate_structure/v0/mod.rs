use crate::consensus::basic::token::{InvalidTokenAmountError, InvalidTokenNoteTooBigError, RandomTokenMintNotAllowedError, TokenNoteOnlyAllowedWhenProposerError};
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::associated_token::token_perpetual_distribution::distribution_function::MAX_DISTRIBUTION_PARAM;
use crate::state_transition::batch_transition::token_mint_transition::v0::v0_methods::TokenMintTransitionV0Methods;
use crate::state_transition::batch_transition::TokenMintTransition;
use crate::tokens::MAX_TOKEN_NOTE_LEN;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;

pub(super) trait TokenMintTransitionActionStructureValidationV0 {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}
impl TokenMintTransitionActionStructureValidationV0 for TokenMintTransition {
    fn validate_structure_v0(&self) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        if self.amount() > MAX_DISTRIBUTION_PARAM || self.amount() == 0 {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ConsensusError::BasicError(BasicError::InvalidTokenAmountError(
                    InvalidTokenAmountError::new(MAX_DISTRIBUTION_PARAM, self.amount()),
                )),
            ));
        }

        // Only a version 1 mint carries a random minimum amount, and no protocol version before
        // 15 decodes one (`StateTransition::active_version_range`), so these checks are inert
        // at every earlier version that selects this generation.
        if let Some(minimum) = self.random_minimum_amount() {
            if minimum == 0 || minimum >= self.amount() {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::BasicError(BasicError::InvalidTokenAmountError(
                        InvalidTokenAmountError::new(self.amount().saturating_sub(1), minimum),
                    )),
                ));
            }
            // Group members agree on an amount, which a random mint does not have
            if self.base().using_group_info().is_some() {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::BasicError(BasicError::RandomTokenMintNotAllowedError(
                        RandomTokenMintNotAllowedError::new(
                            self.base().token_id(),
                            "a group action cannot mint a random amount".to_string(),
                        ),
                    )),
                ));
            }
            // The payout lands after the block executes the mint, so it must not depend on the
            // token's configuration read then
            if self.issued_to_identity_id().is_none() {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::BasicError(BasicError::RandomTokenMintNotAllowedError(
                        RandomTokenMintNotAllowedError::new(
                            self.base().token_id(),
                            "a random mint must name its recipient".to_string(),
                        ),
                    )),
                ));
            }
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
            if let Some(group_state_transition_info) = self.base().using_group_info() {
                if !group_state_transition_info.action_is_proposer {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ConsensusError::BasicError(
                            BasicError::TokenNoteOnlyAllowedWhenProposerError(
                                TokenNoteOnlyAllowedWhenProposerError::new(),
                            ),
                        ),
                    ));
                }
            }
        }
        Ok(SimpleConsensusValidationResult::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::group::GroupStateTransitionInfo;
    use crate::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
    use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
    use crate::state_transition::batch_transition::token_mint_transition::{
        TokenMintTransitionV0, TokenMintTransitionV1,
    };
    use platform_value::Identifier;

    fn make_base() -> TokenBaseTransition {
        TokenBaseTransition::V0(TokenBaseTransitionV0 {
            identity_contract_nonce: 1,
            token_contract_position: 0,
            data_contract_id: Identifier::default(),
            token_id: Identifier::new([1u8; 32]),
            using_group_info: None,
        })
    }

    fn make_base_with_group(is_proposer: bool) -> TokenBaseTransition {
        TokenBaseTransition::V0(TokenBaseTransitionV0 {
            identity_contract_nonce: 1,
            token_contract_position: 0,
            data_contract_id: Identifier::default(),
            token_id: Identifier::new([1u8; 32]),
            using_group_info: Some(GroupStateTransitionInfo {
                group_contract_position: 0,
                action_id: Identifier::new([10u8; 32]),
                action_is_proposer: is_proposer,
            }),
        })
    }

    fn make_valid_mint() -> TokenMintTransition {
        TokenMintTransition::V0(TokenMintTransitionV0 {
            base: make_base(),
            issued_to_identity_id: None,
            amount: 500,
            public_note: None,
        })
    }

    #[test]
    fn valid_mint_passes() {
        let transition = make_valid_mint();
        let result = transition.validate_structure_v0().unwrap();
        assert!(result.is_valid(), "expected no errors: {:?}", result.errors);
    }

    #[test]
    fn zero_amount_returns_invalid_token_amount_error() {
        let transition = TokenMintTransition::V0(TokenMintTransitionV0 {
            base: make_base(),
            issued_to_identity_id: None,
            amount: 0,
            public_note: None,
        });
        let result = transition.validate_structure_v0().unwrap();
        assert_eq!(result.errors.len(), 1);
        assert!(matches!(
            &result.errors[0],
            ConsensusError::BasicError(BasicError::InvalidTokenAmountError(_))
        ));
    }

    #[test]
    fn amount_exceeding_max_returns_invalid_token_amount_error() {
        let transition = TokenMintTransition::V0(TokenMintTransitionV0 {
            base: make_base(),
            issued_to_identity_id: None,
            amount: MAX_DISTRIBUTION_PARAM + 1,
            public_note: None,
        });
        let result = transition.validate_structure_v0().unwrap();
        assert_eq!(result.errors.len(), 1);
        assert!(matches!(
            &result.errors[0],
            ConsensusError::BasicError(BasicError::InvalidTokenAmountError(_))
        ));
    }

    #[test]
    fn amount_at_max_distribution_param_passes() {
        let transition = TokenMintTransition::V0(TokenMintTransitionV0 {
            base: make_base(),
            issued_to_identity_id: None,
            amount: MAX_DISTRIBUTION_PARAM,
            public_note: None,
        });
        let result = transition.validate_structure_v0().unwrap();
        assert!(result.is_valid(), "expected no errors: {:?}", result.errors);
    }

    #[test]
    fn public_note_too_big_returns_error() {
        let transition = TokenMintTransition::V0(TokenMintTransitionV0 {
            base: make_base(),
            issued_to_identity_id: None,
            amount: 100,
            public_note: Some("x".repeat(MAX_TOKEN_NOTE_LEN + 1)),
        });
        let result = transition.validate_structure_v0().unwrap();
        assert_eq!(result.errors.len(), 1);
        assert!(matches!(
            &result.errors[0],
            ConsensusError::BasicError(BasicError::InvalidTokenNoteTooBigError(_))
        ));
    }

    #[test]
    fn public_note_at_max_length_passes() {
        let transition = TokenMintTransition::V0(TokenMintTransitionV0 {
            base: make_base(),
            issued_to_identity_id: None,
            amount: 100,
            public_note: Some("x".repeat(MAX_TOKEN_NOTE_LEN)),
        });
        let result = transition.validate_structure_v0().unwrap();
        assert!(result.is_valid(), "expected no errors: {:?}", result.errors);
    }

    #[test]
    fn note_on_non_proposer_returns_error() {
        let transition = TokenMintTransition::V0(TokenMintTransitionV0 {
            base: make_base_with_group(false),
            issued_to_identity_id: None,
            amount: 100,
            public_note: Some("a valid note".to_string()),
        });
        let result = transition.validate_structure_v0().unwrap();
        assert_eq!(result.errors.len(), 1);
        assert!(matches!(
            &result.errors[0],
            ConsensusError::BasicError(BasicError::TokenNoteOnlyAllowedWhenProposerError(_))
        ));
    }

    #[test]
    fn note_on_proposer_passes() {
        let transition = TokenMintTransition::V0(TokenMintTransitionV0 {
            base: make_base_with_group(true),
            issued_to_identity_id: None,
            amount: 100,
            public_note: Some("a valid note".to_string()),
        });
        let result = transition.validate_structure_v0().unwrap();
        assert!(result.is_valid(), "expected no errors: {:?}", result.errors);
    }

    #[test]
    fn note_without_group_info_passes() {
        let transition = TokenMintTransition::V0(TokenMintTransitionV0 {
            base: make_base(),
            issued_to_identity_id: None,
            amount: 100,
            public_note: Some("a valid note".to_string()),
        });
        let result = transition.validate_structure_v0().unwrap();
        assert!(result.is_valid(), "expected no errors: {:?}", result.errors);
    }

    fn random_mint(
        base: TokenBaseTransition,
        issued_to_identity_id: Option<Identifier>,
        amount: u64,
        random_minimum_amount: Option<u64>,
    ) -> TokenMintTransition {
        TokenMintTransition::V1(TokenMintTransitionV1 {
            base,
            issued_to_identity_id,
            amount,
            public_note: None,
            random_minimum_amount,
        })
    }

    fn recipient() -> Option<Identifier> {
        Some(Identifier::new([4u8; 32]))
    }

    #[test]
    fn should_accept_a_random_mint_from_one_to_below_the_amount() {
        for minimum in [1, 250, 499] {
            let result = random_mint(make_base(), recipient(), 500, Some(minimum))
                .validate_structure_v0()
                .unwrap();
            assert!(result.is_valid(), "{minimum}: {:?}", result.errors);
        }
        // A version 1 mint without a random amount is a plain mint
        let result = random_mint(make_base(), None, 500, None)
            .validate_structure_v0()
            .unwrap();
        assert!(result.is_valid(), "expected no errors: {:?}", result.errors);
    }

    #[test]
    fn should_refuse_a_random_mint_minimum_of_zero_or_not_below_the_amount() {
        for minimum in [0, 500, 501] {
            let result = random_mint(make_base(), recipient(), 500, Some(minimum))
                .validate_structure_v0()
                .unwrap();
            assert_eq!(result.errors.len(), 1);
            assert!(matches!(
                &result.errors[0],
                ConsensusError::BasicError(BasicError::InvalidTokenAmountError(e))
                    if e.token_amount() == minimum
            ));
        }
    }

    #[test]
    fn should_refuse_a_random_mint_as_a_group_action() {
        for is_proposer in [true, false] {
            let result = random_mint(make_base_with_group(is_proposer), recipient(), 500, Some(1))
                .validate_structure_v0()
                .unwrap();
            assert_eq!(result.errors.len(), 1);
            assert!(matches!(
                &result.errors[0],
                ConsensusError::BasicError(BasicError::RandomTokenMintNotAllowedError(e))
                    if e.token_id() == Identifier::new([1u8; 32])
            ));
        }
        // A group mint of a fixed amount stays allowed
        let result = random_mint(make_base_with_group(true), recipient(), 500, None)
            .validate_structure_v0()
            .unwrap();
        assert!(result.is_valid(), "expected no errors: {:?}", result.errors);
    }

    #[test]
    fn should_refuse_a_random_mint_without_a_recipient() {
        let result = random_mint(make_base(), None, 500, Some(1))
            .validate_structure_v0()
            .unwrap();
        assert_eq!(result.errors.len(), 1);
        assert!(matches!(
            &result.errors[0],
            ConsensusError::BasicError(BasicError::RandomTokenMintNotAllowedError(_))
        ));
    }
}
