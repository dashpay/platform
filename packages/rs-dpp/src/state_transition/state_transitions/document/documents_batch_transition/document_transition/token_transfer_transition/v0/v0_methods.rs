use platform_value::Identifier;

use crate::state_transition::documents_batch_transition::document_transition::token_transfer_transition::TokenTransferTransitionV0;
use crate::state_transition::documents_batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::documents_batch_transition::token_base_transition::TokenBaseTransition;

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

pub trait TokenTransferTransitionV0Methods: TokenBaseTransitionAccessors {
    /// Returns a reference to the `revision` field of the `DocumentReplaceTransitionV0`.
    fn amount(&self) -> u64;

    /// Sets the value of the `revision` field in the `DocumentReplaceTransitionV0`.
    fn set_amount(&mut self, amount: u64);

    /// Returns the `recipient_owner_id` field of the `DocumentReplaceTransitionV0`.
    fn recipient_owner_id(&self) -> Identifier;

    /// Returns a reference to the `recipient_owner_id` field of the `DocumentReplaceTransitionV0`.
    fn recipient_owner_id_ref(&self) -> &Identifier;

    /// Sets the value of the `recipient_owner_id` field in the `DocumentReplaceTransitionV0`.
    fn set_recipient_owner_id(&mut self, recipient_owner_id: Identifier);
}

impl TokenTransferTransitionV0Methods for TokenTransferTransitionV0 {
    fn amount(&self) -> u64 {
        self.amount
    }

    fn set_amount(&mut self, amount: u64) {
        self.amount = amount;
    }

    fn recipient_owner_id(&self) -> Identifier {
        self.recipient_owner_id
    }

    fn recipient_owner_id_ref(&self) -> &Identifier {
        &self.recipient_owner_id
    }

    fn set_recipient_owner_id(&mut self, recipient_owner_id: Identifier) {
        self.recipient_owner_id = recipient_owner_id;
    }
}
