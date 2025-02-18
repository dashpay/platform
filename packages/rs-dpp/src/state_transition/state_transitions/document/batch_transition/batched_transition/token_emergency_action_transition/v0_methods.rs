use platform_value::Identifier;
use crate::prelude::IdentityNonce;
use crate::state_transition::batch_transition::batched_transition::multi_party_action::AllowedAsMultiPartyAction;
use crate::state_transition::batch_transition::batched_transition::token_emergency_action_transition::v0::v0_methods::TokenEmergencyActionTransitionV0Methods;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::TokenEmergencyActionTransition;
use crate::tokens::emergency_action::TokenEmergencyAction;
use crate::util::hash::hash_double;

impl TokenBaseTransitionAccessors for TokenEmergencyActionTransition {
    fn base(&self) -> &TokenBaseTransition {
        match self {
            TokenEmergencyActionTransition::V0(v0) => &v0.base,
        }
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        match self {
            TokenEmergencyActionTransition::V0(v0) => &mut v0.base,
        }
    }

    fn set_base(&mut self, base: TokenBaseTransition) {
        match self {
            TokenEmergencyActionTransition::V0(v0) => v0.base = base,
        }
    }
}

impl TokenEmergencyActionTransitionV0Methods for TokenEmergencyActionTransition {
    fn public_note(&self) -> Option<&String> {
        match self {
            TokenEmergencyActionTransition::V0(v0) => v0.public_note(),
        }
    }

    fn public_note_owned(self) -> Option<String> {
        match self {
            TokenEmergencyActionTransition::V0(v0) => v0.public_note_owned(),
        }
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        match self {
            TokenEmergencyActionTransition::V0(v0) => v0.set_public_note(public_note),
        }
    }

    fn emergency_action(&self) -> TokenEmergencyAction {
        match self {
            TokenEmergencyActionTransition::V0(v0) => v0.emergency_action(),
        }
    }

    fn set_emergency_action(&mut self, emergency_action: TokenEmergencyAction) {
        match self {
            TokenEmergencyActionTransition::V0(v0) => v0.set_emergency_action(emergency_action),
        }
    }
}

impl AllowedAsMultiPartyAction for TokenEmergencyActionTransition {
    fn calculate_action_id(&self, owner_id: Identifier) -> Identifier {
        match self {
            TokenEmergencyActionTransition::V0(v0) => v0.calculate_action_id(owner_id),
        }
    }
}

impl TokenEmergencyActionTransition {
    pub fn calculate_action_id_with_fields(
        token_id: &[u8; 32],
        owner_id: &[u8; 32],
        identity_contract_nonce: IdentityNonce,
        emergency_action: TokenEmergencyAction,
    ) -> Identifier {
        let mut bytes = b"action_token_emergency_action".to_vec();
        bytes.extend_from_slice(token_id);
        bytes.extend_from_slice(owner_id);
        bytes.extend_from_slice(&identity_contract_nonce.to_be_bytes());
        bytes.extend_from_slice(&[emergency_action as u8]);

        hash_double(bytes).into()
    }
}
