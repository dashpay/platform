use platform_value::Identifier;
use crate::consensus::basic::BasicError;
use crate::consensus::basic::token::{InvalidTokenNoteTooBigError, TokenTransferToOurselfError};
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use crate::state_transition::batch_transition::batched_transition::validation::{validate_public_note, validate_token_amount_v0};
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
        let mut result = validate_token_amount_v0(self.amount());

        if self.recipient_id() == owner_id {
            result.add_error(TokenTransferToOurselfError::new(
                self.base().token_id(),
                owner_id,
            ));
        }

        if let Some(public_note) = self.public_note() {
            result.merge(validate_public_note(public_note));
        }

        if let Some(shared_encrypted_note) = self.shared_encrypted_note() {
            if shared_encrypted_note.2.len() > MAX_TOKEN_NOTE_LEN {
                result.add_error(InvalidTokenNoteTooBigError::new(
                    MAX_TOKEN_NOTE_LEN as u32,
                    "shared_encrypted_note",
                    shared_encrypted_note.2.len() as u32,
                ));
            }
        }

        if let Some(private_encrypted_note) = self.private_encrypted_note() {
            if private_encrypted_note.2.len() > MAX_TOKEN_NOTE_LEN {
                result.add_error(InvalidTokenNoteTooBigError::new(
                    MAX_TOKEN_NOTE_LEN as u32,
                    "private_encrypted_note",
                    private_encrypted_note.2.len() as u32,
                ));
            }
        }

        Ok(result)
    }
}
