use platform_value::Identifier;
use crate::prelude::{DerivationEncryptionKeyIndex, RecipientKeyIndex, RootEncryptionKeyIndex, SenderKeyIndex};
use crate::state_transition::batch_transition::batched_transition::token_transfer_transition::v0::v0_methods::TokenTransferTransitionV0Methods;
use crate::state_transition::batch_transition::batched_transition::token_transfer_transition::{TokenTransferTransitionV0, TokenTransferTransitionV1};
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::TokenTransferTransition;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::tokens::{PrivateEncryptedNote, SharedEncryptedNote};

impl TokenBaseTransitionAccessors for TokenTransferTransition {
    fn base(&self) -> &TokenBaseTransition {
        match self {
            TokenTransferTransition::V0(v0) => &v0.base,
            TokenTransferTransition::V1(v1) => &v1.base,
        }
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        match self {
            TokenTransferTransition::V0(v0) => &mut v0.base,
            TokenTransferTransition::V1(v1) => &mut v1.base,
        }
    }

    fn set_base(&mut self, base: TokenBaseTransition) {
        match self {
            TokenTransferTransition::V0(v0) => v0.base = base,
            TokenTransferTransition::V1(v1) => v1.base = base,
        }
    }
}

impl TokenTransferTransition {
    /// Sets the least the transfer moves when its amount is random, taking the format that
    /// holds it: `Some` turns a version 0 transfer into a version 1 one, and `None` turns a
    /// version 1 transfer back into version 0, the format every protocol version accepts. Only
    /// the versioned transfer has the setter: a bare `TokenTransferTransitionV0` holds no random
    /// amount, so a setter on it could only drop the value.
    pub fn set_random_minimum_amount(&mut self, random_minimum_amount: Option<u64>) {
        let current = std::mem::replace(
            self,
            TokenTransferTransition::V0(TokenTransferTransitionV0::default()),
        );
        *self = match (current, random_minimum_amount) {
            (TokenTransferTransition::V0(v0), None) => TokenTransferTransition::V0(v0),
            (TokenTransferTransition::V0(v0), Some(minimum)) => {
                TokenTransferTransition::V1(TokenTransferTransitionV1::from_v0(v0, Some(minimum)))
            }
            (TokenTransferTransition::V1(mut v1), Some(minimum)) => {
                v1.random_minimum_amount = Some(minimum);
                TokenTransferTransition::V1(v1)
            }
            (TokenTransferTransition::V1(v1), None) => {
                TokenTransferTransition::V0(v1.into_v0_and_random_minimum_amount().0)
            }
        };
    }
}

impl TokenTransferTransitionV0Methods for TokenTransferTransition {
    fn amount(&self) -> u64 {
        match self {
            TokenTransferTransition::V0(v0) => v0.amount,
            TokenTransferTransition::V1(v1) => v1.amount,
        }
    }

    fn set_amount(&mut self, amount: u64) {
        match self {
            TokenTransferTransition::V0(v0) => v0.amount = amount,
            TokenTransferTransition::V1(v1) => v1.amount = amount,
        }
    }

    fn random_minimum_amount(&self) -> Option<u64> {
        match self {
            TokenTransferTransition::V0(_) => None,
            TokenTransferTransition::V1(v1) => v1.random_minimum_amount,
        }
    }

    fn recipient_id(&self) -> Identifier {
        match self {
            TokenTransferTransition::V0(v0) => v0.recipient_id,
            TokenTransferTransition::V1(v1) => v1.recipient_id,
        }
    }

    fn recipient_id_ref(&self) -> &Identifier {
        match self {
            TokenTransferTransition::V0(v0) => &v0.recipient_id,
            TokenTransferTransition::V1(v1) => &v1.recipient_id,
        }
    }

    fn set_recipient_id(&mut self, recipient_id: Identifier) {
        match self {
            TokenTransferTransition::V0(v0) => v0.recipient_id = recipient_id,
            TokenTransferTransition::V1(v1) => v1.recipient_id = recipient_id,
        }
    }

    // Methods for `public_note`
    fn public_note(&self) -> Option<&String> {
        match self {
            TokenTransferTransition::V0(v0) => v0.public_note.as_ref(),
            TokenTransferTransition::V1(v1) => v1.public_note.as_ref(),
        }
    }

    fn public_note_owned(self) -> Option<String> {
        match self {
            TokenTransferTransition::V0(v0) => v0.public_note,
            TokenTransferTransition::V1(v1) => v1.public_note,
        }
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        match self {
            TokenTransferTransition::V0(v0) => v0.public_note = public_note,
            TokenTransferTransition::V1(v1) => v1.public_note = public_note,
        }
    }

    fn shared_encrypted_note(&self) -> Option<&SharedEncryptedNote> {
        match self {
            TokenTransferTransition::V0(v0) => v0.shared_encrypted_note.as_ref(),
            TokenTransferTransition::V1(v1) => v1.shared_encrypted_note.as_ref(),
        }
    }

    fn shared_encrypted_note_owned(self) -> Option<SharedEncryptedNote> {
        match self {
            TokenTransferTransition::V0(v0) => v0.shared_encrypted_note,
            TokenTransferTransition::V1(v1) => v1.shared_encrypted_note,
        }
    }

    fn set_shared_encrypted_note(&mut self, shared_encrypted_note: Option<SharedEncryptedNote>) {
        match self {
            TokenTransferTransition::V0(v0) => v0.shared_encrypted_note = shared_encrypted_note,
            TokenTransferTransition::V1(v1) => v1.shared_encrypted_note = shared_encrypted_note,
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
            TokenTransferTransition::V1(v1) => v1.private_encrypted_note.as_ref(),
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
            TokenTransferTransition::V1(v1) => v1.private_encrypted_note,
        }
    }

    fn set_private_encrypted_note(&mut self, private_encrypted_note: Option<PrivateEncryptedNote>) {
        match self {
            TokenTransferTransition::V0(v0) => v0.private_encrypted_note = private_encrypted_note,
            TokenTransferTransition::V1(v1) => v1.private_encrypted_note = private_encrypted_note,
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
            TokenTransferTransition::V1(v1) => (
                v1.public_note,
                v1.shared_encrypted_note,
                v1.private_encrypted_note,
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
            TokenTransferTransition::V1(v1) => (
                v1.public_note.clone(),
                v1.shared_encrypted_note.clone(),
                v1.private_encrypted_note.clone(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;

    fn transfer_v0() -> TokenTransferTransition {
        TokenTransferTransition::V0(TokenTransferTransitionV0 {
            base: TokenBaseTransition::V0(TokenBaseTransitionV0 {
                identity_contract_nonce: 4,
                token_contract_position: 1,
                data_contract_id: Identifier::new([1; 32]),
                token_id: Identifier::new([2; 32]),
                using_group_info: None,
            }),
            amount: 500,
            recipient_id: Identifier::new([3; 32]),
            public_note: Some("note".to_string()),
            shared_encrypted_note: None,
            private_encrypted_note: Some((1, 2, vec![9])),
        })
    }

    #[test]
    fn should_take_the_format_that_holds_the_random_minimum_amount() {
        let original = transfer_v0();
        let mut transfer = original.clone();
        assert_eq!(transfer.random_minimum_amount(), None);

        transfer.set_random_minimum_amount(None);
        assert_eq!(transfer, original, "a version 0 transfer stays as it is");

        transfer.set_random_minimum_amount(Some(10));
        assert!(matches!(transfer, TokenTransferTransition::V1(_)));
        assert_eq!(transfer.random_minimum_amount(), Some(10));
        assert_eq!(transfer.amount(), 500);
        assert_eq!(transfer.recipient_id(), Identifier::new([3; 32]));
        assert_eq!(transfer.notes(), original.notes());
        assert_eq!(transfer.base(), original.base());

        transfer.set_random_minimum_amount(Some(20));
        assert_eq!(transfer.random_minimum_amount(), Some(20));

        transfer.set_random_minimum_amount(None);
        assert_eq!(
            transfer, original,
            "without a random amount the transfer is version 0 again"
        );
    }
}
