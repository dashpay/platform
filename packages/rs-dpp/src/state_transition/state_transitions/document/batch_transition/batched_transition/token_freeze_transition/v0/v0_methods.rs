use platform_value::Identifier;
use crate::state_transition::batch_transition::batched_transition::multi_party_action::AllowedAsMultiPartyAction;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use crate::state_transition::batch_transition::token_freeze_transition::TokenFreezeTransitionV0;
use crate::state_transition::batch_transition::TokenFreezeTransition;

impl TokenBaseTransitionAccessors for TokenFreezeTransitionV0 {
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

pub trait TokenFreezeTransitionV0Methods:
    TokenBaseTransitionAccessors + AllowedAsMultiPartyAction
{
    /// Returns the `public_note` field of the `TokenFreezeTransitionV0`.
    fn public_note(&self) -> Option<&String>;

    /// Returns the owned `public_note` field of the `TokenFreezeTransitionV0`.
    fn public_note_owned(self) -> Option<String>;

    /// Sets the value of the `public_note` field in the `TokenFreezeTransitionV0`.
    fn set_public_note(&mut self, public_note: Option<String>);

    /// Returns the `frozen_identity_id` field of the `TokenFreezeTransitionV0`.
    fn frozen_identity_id(&self) -> Identifier;

    /// Sets the value of the `frozen_identity_id` field in the `TokenFreezeTransitionV0`.
    fn set_frozen_identity_id(&mut self, frozen_identity_id: Identifier);
}

impl TokenFreezeTransitionV0Methods for TokenFreezeTransitionV0 {
    fn public_note(&self) -> Option<&String> {
        self.public_note.as_ref()
    }

    fn public_note_owned(self) -> Option<String> {
        self.public_note
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        self.public_note = public_note;
    }

    fn frozen_identity_id(&self) -> Identifier {
        self.identity_to_freeze_id
    }
    fn set_frozen_identity_id(&mut self, frozen_identity_id: Identifier) {
        self.identity_to_freeze_id = frozen_identity_id;
    }
}

impl AllowedAsMultiPartyAction for TokenFreezeTransitionV0 {
    fn calculate_action_id(&self, owner_id: Identifier) -> Identifier {
        let TokenFreezeTransitionV0 {
            base,
            identity_to_freeze_id: frozen_identity_id,
            ..
        } = self;

        TokenFreezeTransition::calculate_action_id_with_fields(
            base.token_id().as_bytes(),
            owner_id.as_bytes(),
            base.identity_contract_nonce(),
            frozen_identity_id.as_bytes(),
        )
    }
}
