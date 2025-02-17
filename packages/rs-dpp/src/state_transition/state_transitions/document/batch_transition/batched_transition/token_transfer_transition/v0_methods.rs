use platform_value::Identifier;
use crate::prelude::{DerivationEncryptionKeyIndex, RecipientKeyIndex, RootEncryptionKeyIndex, SenderKeyIndex};
use crate::state_transition::batch_transition::batched_transition::token_transfer_transition::v0::v0_methods::TokenTransferTransitionV0Methods;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::TokenTransferTransition;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::tokens::{PrivateEncryptedNote, SharedEncryptedNote};

impl TokenBaseTransitionAccessors for TokenTransferTransition {
    fn base(&self) -> &TokenBaseTransition {
        match self {
            TokenTransferTransition::V0(v0) => &v0.base,
        }
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        match self {
            TokenTransferTransition::V0(v0) => &mut v0.base,
        }
    }

    fn set_base(&mut self, base: TokenBaseTransition) {
        match self {
            TokenTransferTransition::V0(v0) => v0.base = base,
        }
    }
}

impl TokenTransferTransitionV0Methods for TokenTransferTransition {
    fn amount(&self) -> u64 {
        match self {
            TokenTransferTransition::V0(v0) => v0.amount,
        }
    }

    fn set_amount(&mut self, amount: u64) {
        match self {
            TokenTransferTransition::V0(v0) => v0.amount = amount,
        }
    }

    fn recipient_id(&self) -> Identifier {
        match self {
            TokenTransferTransition::V0(v0) => v0.recipient_id,
        }
    }

    fn recipient_id_ref(&self) -> &Identifier {
        match self {
            TokenTransferTransition::V0(v0) => &v0.recipient_id,
        }
    }

    fn set_recipient_id(&mut self, recipient_id: Identifier) {
        match self {
            TokenTransferTransition::V0(v0) => v0.recipient_id = recipient_id,
        }
    }

    // Methods for `public_note`
    fn public_note(&self) -> Option<&String> {
        match self {
            TokenTransferTransition::V0(v0) => v0.public_note.as_ref(),
        }
    }

    fn public_note_owned(self) -> Option<String> {
        match self {
            TokenTransferTransition::V0(v0) => v0.public_note,
        }
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        match self {
            TokenTransferTransition::V0(v0) => v0.public_note = public_note,
        }
    }

    fn shared_encrypted_note(&self) -> Option<&SharedEncryptedNote> {
        match self {
            TokenTransferTransition::V0(v0) => v0.shared_encrypted_note.as_ref(),
        }
    }

    fn shared_encrypted_note_owned(self) -> Option<SharedEncryptedNote> {
        match self {
            TokenTransferTransition::V0(v0) => v0.shared_encrypted_note,
        }
    }

    fn set_shared_encrypted_note(&mut self, shared_encrypted_note: Option<SharedEncryptedNote>) {
        match self {
            TokenTransferTransition::V0(v0) => v0.shared_encrypted_note = shared_encrypted_note,
        }
    }

    fn private_encrypted_note(
        &self,
    ) -> Option<&(
        RootEncryptionKeyIndex,
        DerivationEncryptionKeyIndex,
        Vec<u8>,
    )> {
        match self {
            TokenTransferTransition::V0(v0) => v0.private_encrypted_note.as_ref(),
        }
    }

    fn private_encrypted_note_owned(
        self,
    ) -> Option<(
        RootEncryptionKeyIndex,
        DerivationEncryptionKeyIndex,
        Vec<u8>,
    )> {
        match self {
            TokenTransferTransition::V0(v0) => v0.private_encrypted_note,
        }
    }

    fn set_private_encrypted_note(&mut self, private_encrypted_note: Option<PrivateEncryptedNote>) {
        match self {
            TokenTransferTransition::V0(v0) => v0.private_encrypted_note = private_encrypted_note,
        }
    }

    // Method to return all notes as owned values
    fn notes_owned(
        self,
    ) -> (
        Option<String>,
        Option<SharedEncryptedNote>,
        Option<PrivateEncryptedNote>,
    ) {
        match self {
            TokenTransferTransition::V0(v0) => (
                v0.public_note,
                v0.shared_encrypted_note,
                v0.private_encrypted_note,
            ),
        }
    }

    fn notes(
        &self,
    ) -> (
        Option<String>,
        Option<(SenderKeyIndex, RecipientKeyIndex, Vec<u8>)>,
        Option<(
            RootEncryptionKeyIndex,
            DerivationEncryptionKeyIndex,
            Vec<u8>,
        )>,
    ) {
        match self {
            TokenTransferTransition::V0(v0) => (
                v0.public_note.clone(),
                v0.shared_encrypted_note.clone(),
                v0.private_encrypted_note.clone(),
            ),
        }
    }
}
