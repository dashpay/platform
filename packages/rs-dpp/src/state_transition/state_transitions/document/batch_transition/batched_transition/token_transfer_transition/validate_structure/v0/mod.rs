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
