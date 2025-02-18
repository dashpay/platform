use platform_value::Identifier;
use crate::state_transition::batch_transition::batched_transition::multi_party_action::AllowedAsMultiPartyAction;
use crate::state_transition::batch_transition::batched_transition::token_emergency_action_transition::TokenEmergencyActionTransitionV0;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use crate::state_transition::batch_transition::TokenEmergencyActionTransition;
use crate::tokens::emergency_action::TokenEmergencyAction;

impl TokenBaseTransitionAccessors for TokenEmergencyActionTransitionV0 {
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

pub trait TokenEmergencyActionTransitionV0Methods:
    TokenBaseTransitionAccessors + AllowedAsMultiPartyAction
{
    /// Returns the `public_note` field of the `TokenEmergencyActionTransitionV0`.
    fn public_note(&self) -> Option<&String>;

    /// Returns the owned `public_note` field of the `TokenEmergencyActionTransitionV0`.
    fn public_note_owned(self) -> Option<String>;

    /// Sets the value of the `public_note` field in the `TokenEmergencyActionTransitionV0`.
    fn set_public_note(&mut self, public_note: Option<String>);

    /// Returns the `emergency_action` field of the `TokenEmergencyActionTransitionV0`.
    fn emergency_action(&self) -> TokenEmergencyAction;

    /// Sets the value of the `emergency_action` field in the `TokenEmergencyActionTransitionV0`.
    fn set_emergency_action(&mut self, emergency_action: TokenEmergencyAction);
}

impl TokenEmergencyActionTransitionV0Methods for TokenEmergencyActionTransitionV0 {
    fn public_note(&self) -> Option<&String> {
        self.public_note.as_ref()
    }

    fn public_note_owned(self) -> Option<String> {
        self.public_note
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        self.public_note = public_note;
    }

    fn emergency_action(&self) -> TokenEmergencyAction {
        self.emergency_action
    }

    fn set_emergency_action(&mut self, emergency_action: TokenEmergencyAction) {
        self.emergency_action = emergency_action;
    }
}

impl AllowedAsMultiPartyAction for TokenEmergencyActionTransitionV0 {
    fn calculate_action_id(&self, owner_id: Identifier) -> Identifier {
        let TokenEmergencyActionTransitionV0 {
            base,
            emergency_action,
            ..
        } = self;

        TokenEmergencyActionTransition::calculate_action_id_with_fields(
            base.token_id().as_bytes(),
            owner_id.as_bytes(),
            base.identity_contract_nonce(),
            *emergency_action,
        )
    }
}
