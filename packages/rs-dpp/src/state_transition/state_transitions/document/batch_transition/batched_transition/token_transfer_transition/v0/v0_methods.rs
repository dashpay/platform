use platform_value::Identifier;
use crate::prelude::{DerivationEncryptionKeyIndex, RecipientKeyIndex, RootEncryptionKeyIndex, SenderKeyIndex};
use crate::state_transition::batch_transition::batched_transition::multi_party_action::AllowedAsMultiPartyAction;
use crate::state_transition::batch_transition::batched_transition::token_transfer_transition::TokenTransferTransitionV0;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use crate::util::hash::hash_double;

impl TokenBaseTransitionAccessors for TokenTransferTransitionV0 {
    fn base(&self) -> &TokenBaseTransition {
        &self.base
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        &mut self.base
    }

    fn set_base(&mut self, base: TokenBaseTransition) {
        self.base = base;
    }
}

pub trait TokenTransferTransitionV0Methods:
    TokenBaseTransitionAccessors + AllowedAsMultiPartyAction
{
    /// Returns the `amount` field of the `TokenTransferTransitionV0`.
    fn amount(&self) -> u64;

    /// Sets the value of the `amount` field in the `TokenTransferTransitionV0`.
    fn set_amount(&mut self, amount: u64);

    /// Returns the `recipient_owner_id` field of the `TokenTransferTransitionV0`.
    fn recipient_id(&self) -> Identifier;

    /// Returns a reference to the `recipient_owner_id` field of the `TokenTransferTransitionV0`.
    fn recipient_id_ref(&self) -> &Identifier;

    /// Sets the value of the `recipient_owner_id` field in the `TokenTransferTransitionV0`.
    fn set_recipient_id(&mut self, recipient_owner_id: Identifier);

    /// Returns the `public_note` field of the `TokenTransferTransitionV0`.
    fn public_note(&self) -> Option<&String>;

    /// Returns the owned `public_note` field of the `TokenTransferTransitionV0`.
    fn public_note_owned(self) -> Option<String>;

    /// Sets the value of the `public_note` field in the `TokenTransferTransitionV0`.
    fn set_public_note(&mut self, public_note: Option<String>);

    /// Returns the `shared_encrypted_note` field of the `TokenTransferTransitionV0`.
    fn shared_encrypted_note(&self) -> Option<&(SenderKeyIndex, RecipientKeyIndex, Vec<u8>)>;

    /// Returns the owned `shared_encrypted_note` field of the `TokenTransferTransitionV0`.
    fn shared_encrypted_note_owned(self) -> Option<(SenderKeyIndex, RecipientKeyIndex, Vec<u8>)>;

    /// Sets the value of the `shared_encrypted_note` field in the `TokenTransferTransitionV0`.
    fn set_shared_encrypted_note(
        &mut self,
        shared_encrypted_note: Option<(SenderKeyIndex, RecipientKeyIndex, Vec<u8>)>,
    );

    /// Returns the `private_encrypted_note` field of the `TokenTransferTransitionV0`.
    fn private_encrypted_note(
        &self,
    ) -> Option<&(
        RootEncryptionKeyIndex,
        DerivationEncryptionKeyIndex,
        Vec<u8>,
    )>;

    /// Returns the owned `private_encrypted_note` field of the `TokenTransferTransitionV0`.
    fn private_encrypted_note_owned(
        self,
    ) -> Option<(
        RootEncryptionKeyIndex,
        DerivationEncryptionKeyIndex,
        Vec<u8>,
    )>;

    /// Sets the value of the `private_encrypted_note` field in the `TokenTransferTransitionV0`.
    fn set_private_encrypted_note(
        &mut self,
        private_encrypted_note: Option<(
            RootEncryptionKeyIndex,
            DerivationEncryptionKeyIndex,
            Vec<u8>,
        )>,
    );

    /// Returns all notes (public, shared, and private) as owned values in a tuple.
    fn notes_owned(
        self,
    ) -> (
        Option<String>,
        Option<(SenderKeyIndex, RecipientKeyIndex, Vec<u8>)>,
        Option<(
            RootEncryptionKeyIndex,
            DerivationEncryptionKeyIndex,
            Vec<u8>,
        )>,
    );
}

impl TokenTransferTransitionV0Methods for TokenTransferTransitionV0 {
    fn amount(&self) -> u64 {
        self.amount
    }

    fn set_amount(&mut self, amount: u64) {
        self.amount = amount;
    }

    fn recipient_id(&self) -> Identifier {
        self.recipient_id
    }

    fn recipient_id_ref(&self) -> &Identifier {
        &self.recipient_id
    }

    fn set_recipient_id(&mut self, recipient_owner_id: Identifier) {
        self.recipient_id = recipient_owner_id;
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

    fn shared_encrypted_note(&self) -> Option<&(SenderKeyIndex, RecipientKeyIndex, Vec<u8>)> {
        self.shared_encrypted_note.as_ref()
    }

    fn shared_encrypted_note_owned(self) -> Option<(SenderKeyIndex, RecipientKeyIndex, Vec<u8>)> {
        self.shared_encrypted_note
    }

    fn set_shared_encrypted_note(
        &mut self,
        shared_encrypted_note: Option<(SenderKeyIndex, RecipientKeyIndex, Vec<u8>)>,
    ) {
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

    fn set_private_encrypted_note(
        &mut self,
        private_encrypted_note: Option<(
            RootEncryptionKeyIndex,
            DerivationEncryptionKeyIndex,
            Vec<u8>,
        )>,
    ) {
        self.private_encrypted_note = private_encrypted_note;
    }

    fn notes_owned(
        self,
    ) -> (
        Option<String>,
        Option<(SenderKeyIndex, RecipientKeyIndex, Vec<u8>)>,
        Option<(
            RootEncryptionKeyIndex,
            DerivationEncryptionKeyIndex,
            Vec<u8>,
        )>,
    ) {
        (
            self.public_note,
            self.shared_encrypted_note,
            self.private_encrypted_note,
        )
    }
}

impl AllowedAsMultiPartyAction for TokenTransferTransitionV0 {
    fn calculate_action_id(&self, owner_id: Identifier) -> Identifier {
        let TokenTransferTransitionV0 {
            base,
            amount,
            recipient_id,
            ..
        } = self;

        let mut bytes = b"action_token_transfer".to_vec();
        bytes.extend_from_slice(base.token_id().as_bytes());
        bytes.extend_from_slice(owner_id.as_bytes());
        bytes.extend_from_slice(recipient_id.as_bytes());
        bytes.extend_from_slice(&base.identity_contract_nonce().to_be_bytes());
        bytes.extend_from_slice(&amount.to_be_bytes());

        hash_double(bytes).into()
    }
}
