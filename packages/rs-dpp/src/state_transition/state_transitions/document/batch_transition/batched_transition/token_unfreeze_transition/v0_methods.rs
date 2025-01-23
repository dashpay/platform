use platform_value::Identifier;
use crate::prelude::IdentityNonce;
use crate::state_transition::batch_transition::batched_transition::multi_party_action::AllowedAsMultiPartyAction;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_unfreeze_transition::v0::v0_methods::TokenUnfreezeTransitionV0Methods;
use crate::state_transition::batch_transition::TokenUnfreezeTransition;
use crate::util::hash::hash_double;

impl TokenBaseTransitionAccessors for TokenUnfreezeTransition {
    fn base(&self) -> &TokenBaseTransition {
        match self {
            TokenUnfreezeTransition::V0(v0) => &v0.base,
        }
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        match self {
            TokenUnfreezeTransition::V0(v0) => &mut v0.base,
        }
    }

    fn set_base(&mut self, base: TokenBaseTransition) {
        match self {
            TokenUnfreezeTransition::V0(v0) => v0.base = base,
        }
    }
}

impl TokenUnfreezeTransitionV0Methods for TokenUnfreezeTransition {
    fn public_note(&self) -> Option<&String> {
        match self {
            TokenUnfreezeTransition::V0(v0) => v0.public_note(),
        }
    }

    fn public_note_owned(self) -> Option<String> {
        match self {
            TokenUnfreezeTransition::V0(v0) => v0.public_note_owned(),
        }
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        match self {
            TokenUnfreezeTransition::V0(v0) => v0.set_public_note(public_note),
        }
    }

    fn frozen_identity_id(&self) -> Identifier {
        match self {
            TokenUnfreezeTransition::V0(v0) => v0.frozen_identity_id(),
        }
    }

    fn set_frozen_identity_id(&mut self, frozen_identity_id: Identifier) {
        match self {
            TokenUnfreezeTransition::V0(v0) => v0.set_frozen_identity_id(frozen_identity_id),
        }
    }
}

impl AllowedAsMultiPartyAction for TokenUnfreezeTransition {
    fn calculate_action_id(&self, owner_id: Identifier) -> Identifier {
        match self {
            TokenUnfreezeTransition::V0(v0) => v0.calculate_action_id(owner_id),
        }
    }
}

impl TokenUnfreezeTransition {
    pub fn calculate_action_id_with_fields(
        token_id: &[u8; 32],
        owner_id: &[u8; 32],
        identity_contract_nonce: IdentityNonce,
        target_id: &[u8; 32],
    ) -> Identifier {
        let mut bytes = b"action_token_unfreeze".to_vec();
        bytes.extend_from_slice(token_id);
        bytes.extend_from_slice(owner_id);
        bytes.extend_from_slice(&identity_contract_nonce.to_be_bytes());
        bytes.extend_from_slice(target_id);

        hash_double(bytes).into()
    }
}
