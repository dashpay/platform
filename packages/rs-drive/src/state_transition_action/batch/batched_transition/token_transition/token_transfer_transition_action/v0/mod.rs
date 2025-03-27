mod transformer;

use std::sync::Arc;

use dpp::identifier::Identifier;
use dpp::prelude::{
    DerivationEncryptionKeyIndex, IdentityNonce, RootEncryptionKeyIndex,
};
use dpp::tokens::{PrivateEncryptedNote, SharedEncryptedNote};
use crate::drive::contract::DataContractFetchInfo;
use crate::state_transition_action::batch::batched_transition::token_transition::token_base_transition_action::{
    TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0,
};

/// Token transfer transition action v0
#[derive(Debug, Clone)]
pub struct TokenTransferTransitionActionV0 {
    /// Base token transition action
    pub base: TokenBaseTransitionAction,
    /// The amount to transfer
    pub amount: u64,
    /// The recipient owner ID
    pub recipient_id: Identifier,
    /// The public note
    pub public_note: Option<String>,
    /// An optional shared encrypted note
    pub shared_encrypted_note: Option<SharedEncryptedNote>,
    /// An optional private encrypted note
    pub private_encrypted_note: Option<(
        RootEncryptionKeyIndex,
        DerivationEncryptionKeyIndex,
        Vec<u8>,
    )>,
}

/// Accessors for `TokenTransferTransitionActionV0`
pub trait TokenTransferTransitionActionAccessorsV0 {
    /// Returns the base token transition action
    fn base(&self) -> &TokenBaseTransitionAction;

    /// Returns the base owned token transition action
    fn base_owned(self) -> TokenBaseTransitionAction;

    /// Returns the amount of tokens to transfer
    fn amount(&self) -> u64;

    /// Returns the recipient owner ID
    fn recipient_id(&self) -> Identifier;

    /// Returns the token position in the contract
    fn token_position(&self) -> u16 {
        self.base().token_position()
    }

    /// Returns the token ID
    fn token_id(&self) -> Identifier {
        self.base().token_id()
    }

    /// Returns the data contract ID from the base action
    fn data_contract_id(&self) -> Identifier {
        self.base().data_contract_id()
    }

    /// Returns a reference to the data contract fetch info from the base action
    fn data_contract_fetch_info_ref(&self) -> &Arc<DataContractFetchInfo> {
        self.base().data_contract_fetch_info_ref()
    }

    /// Returns the data contract fetch info
    fn data_contract_fetch_info(&self) -> Arc<DataContractFetchInfo> {
        self.base().data_contract_fetch_info()
    }

    /// Returns the identity contract nonce from the base action
    fn identity_contract_nonce(&self) -> IdentityNonce {
        self.base().identity_contract_nonce()
    }

    /// Returns the public note, if present
    fn public_note(&self) -> Option<&String>;

    /// Consumes the `TokenTransferTransitionActionV0` and returns the public note, if present
    fn public_note_owned(self) -> Option<String>;

    /// Sets the public note
    fn set_public_note(&mut self, public_note: Option<String>);

    /// Returns the shared encrypted note, if present
    fn shared_encrypted_note(&self) -> Option<&SharedEncryptedNote>;

    /// Consumes the `TokenTransferTransitionActionV0` and returns the shared encrypted note, if present
    fn shared_encrypted_note_owned(self) -> Option<SharedEncryptedNote>;

    /// Sets the shared encrypted note
    fn set_shared_encrypted_note(&mut self, shared_encrypted_note: Option<SharedEncryptedNote>);

    /// Returns the private encrypted note, if present
    fn private_encrypted_note(
        &self,
    ) -> Option<&(
        RootEncryptionKeyIndex,
        DerivationEncryptionKeyIndex,
        Vec<u8>,
    )>;

    /// Consumes the `TokenTransferTransitionActionV0` and returns the private encrypted note, if present
    fn private_encrypted_note_owned(
        self,
    ) -> Option<(
        RootEncryptionKeyIndex,
        DerivationEncryptionKeyIndex,
        Vec<u8>,
    )>;

    /// Sets the private encrypted note
    fn set_private_encrypted_note(&mut self, private_encrypted_note: Option<PrivateEncryptedNote>);

    /// All notes
    fn notes_owned(
        self,
    ) -> (
        Option<String>,
        Option<SharedEncryptedNote>,
        Option<PrivateEncryptedNote>,
    );
    /// All notes
    fn notes(
        &self,
    ) -> (
        Option<String>,
        Option<SharedEncryptedNote>,
        Option<PrivateEncryptedNote>,
    );
}

impl TokenTransferTransitionActionAccessorsV0 for TokenTransferTransitionActionV0 {
    fn base(&self) -> &TokenBaseTransitionAction {
        &self.base
    }

    fn base_owned(self) -> TokenBaseTransitionAction {
        self.base
    }

    fn amount(&self) -> u64 {
        self.amount
    }

    fn recipient_id(&self) -> Identifier {
        self.recipient_id
    }

    fn public_note(&self) -> Option<&String> {
        self.public_note.as_ref()
    }

    fn public_note_owned(self) -> Option<String> {
        self.public_note
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        self.public_note = public_note;
    }

    fn shared_encrypted_note(&self) -> Option<&SharedEncryptedNote> {
        self.shared_encrypted_note.as_ref()
    }

    fn shared_encrypted_note_owned(self) -> Option<SharedEncryptedNote> {
        self.shared_encrypted_note
    }

    fn set_shared_encrypted_note(&mut self, shared_encrypted_note: Option<SharedEncryptedNote>) {
        self.shared_encrypted_note = shared_encrypted_note;
    }

    fn private_encrypted_note(
        &self,
    ) -> Option<&(
        RootEncryptionKeyIndex,
        DerivationEncryptionKeyIndex,
        Vec<u8>,
    )> {
        self.private_encrypted_note.as_ref()
    }

    fn private_encrypted_note_owned(
        self,
    ) -> Option<(
        RootEncryptionKeyIndex,
        DerivationEncryptionKeyIndex,
        Vec<u8>,
    )> {
        self.private_encrypted_note
    }

    fn set_private_encrypted_note(&mut self, private_encrypted_note: Option<PrivateEncryptedNote>) {
        self.private_encrypted_note = private_encrypted_note;
    }

    fn notes_owned(
        self,
    ) -> (
        Option<String>,
        Option<SharedEncryptedNote>,
        Option<PrivateEncryptedNote>,
    ) {
        (
            self.public_note,
            self.shared_encrypted_note,
            self.private_encrypted_note,
        )
    }

    fn notes(
        &self,
    ) -> (
        Option<String>,
        Option<SharedEncryptedNote>,
        Option<PrivateEncryptedNote>,
    ) {
        (
            self.public_note.clone(),
            self.shared_encrypted_note.clone(),
            self.private_encrypted_note.clone(),
        )
    }
}
