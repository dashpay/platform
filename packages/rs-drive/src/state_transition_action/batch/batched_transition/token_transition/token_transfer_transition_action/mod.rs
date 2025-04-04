use derive_more::From;

use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::TokenBaseTransitionAction;
use dpp::identifier::Identifier;
use dpp::prelude::{DerivationEncryptionKeyIndex, RootEncryptionKeyIndex};
use dpp::tokens::{PrivateEncryptedNote, SharedEncryptedNote};

/// transformer module
pub mod transformer;
/// v0
pub mod v0;

pub use v0::*;

/// TokenTransferTransitionAction
#[derive(Debug, Clone, From)]
pub enum TokenTransferTransitionAction {
    /// v0
    V0(TokenTransferTransitionActionV0),
}

impl TokenTransferTransitionActionAccessorsV0 for TokenTransferTransitionAction {
    fn base(&self) -> &TokenBaseTransitionAction {
        match self {
            TokenTransferTransitionAction::V0(v0) => v0.base(),
        }
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        match self {
            TokenTransferTransitionAction::V0(v0) => v0.base_owned(),
        }
    }

    fn amount(&self) -> u64 {
        match self {
            TokenTransferTransitionAction::V0(v0) => v0.amount(),
        }
    }

    fn recipient_id(&self) -> Identifier {
        match self {
            TokenTransferTransitionAction::V0(v0) => v0.recipient_id(),
        }
    }

    fn public_note(&self) -> Option<&String> {
        match self {
            TokenTransferTransitionAction::V0(v0) => v0.public_note(),
        }
    }

    fn public_note_owned(self) -> Option<String> {
        match self {
            TokenTransferTransitionAction::V0(v0) => v0.public_note_owned(),
        }
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        match self {
            TokenTransferTransitionAction::V0(v0) => v0.set_public_note(public_note),
        }
    }

    fn shared_encrypted_note(&self) -> Option<&SharedEncryptedNote> {
        match self {
            TokenTransferTransitionAction::V0(v0) => v0.shared_encrypted_note(),
        }
    }

    fn shared_encrypted_note_owned(self) -> Option<SharedEncryptedNote> {
        match self {
            TokenTransferTransitionAction::V0(v0) => v0.shared_encrypted_note_owned(),
        }
    }

    fn set_shared_encrypted_note(&mut self, shared_encrypted_note: Option<SharedEncryptedNote>) {
        match self {
            TokenTransferTransitionAction::V0(v0) => {
                v0.set_shared_encrypted_note(shared_encrypted_note)
            }
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
            TokenTransferTransitionAction::V0(v0) => v0.private_encrypted_note(),
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
            TokenTransferTransitionAction::V0(v0) => v0.private_encrypted_note_owned(),
        }
    }

    fn set_private_encrypted_note(&mut self, private_encrypted_note: Option<PrivateEncryptedNote>) {
        match self {
            TokenTransferTransitionAction::V0(v0) => {
                v0.set_private_encrypted_note(private_encrypted_note)
            }
        }
    }

    fn notes(
        &self,
    ) -> (
        Option<String>,
        Option<SharedEncryptedNote>,
        Option<PrivateEncryptedNote>,
    ) {
        match self {
            TokenTransferTransitionAction::V0(v0) => v0.notes(),
        }
    }

    fn notes_owned(
        self,
    ) -> (
        Option<String>,
        Option<SharedEncryptedNote>,
        Option<PrivateEncryptedNote>,
    ) {
        match self {
            TokenTransferTransitionAction::V0(v0) => v0.notes_owned(),
        }
    }
}
