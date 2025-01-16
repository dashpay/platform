use platform_value::Identifier;
use crate::prelude::IdentityNonce;
use crate::state_transition::batch_transition::batched_transition::multi_party_action::AllowedAsMultiPartyAction;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_destroy_frozen_funds_transition::TokenDestroyFrozenFundsTransition;
use crate::state_transition::batch_transition::token_destroy_frozen_funds_transition::v0::v0_methods::TokenDestroyFrozenFundsTransitionV0Methods;
use crate::util::hash::hash_double;

impl TokenBaseTransitionAccessors for TokenDestroyFrozenFundsTransition {
    fn base(&self) -> &TokenBaseTransition {
        match self {
            TokenDestroyFrozenFundsTransition::V0(v0) => &v0.base,
        }
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        match self {
            TokenDestroyFrozenFundsTransition::V0(v0) => &mut v0.base,
        }
    }

    fn set_base(&mut self, base: TokenBaseTransition) {
        match self {
            TokenDestroyFrozenFundsTransition::V0(v0) => v0.base = base,
        }
    }
}

impl TokenDestroyFrozenFundsTransitionV0Methods for TokenDestroyFrozenFundsTransition {
    fn public_note(&self) -> Option<&String> {
        match self {
            TokenDestroyFrozenFundsTransition::V0(v0) => v0.public_note(),
        }
    }

    fn public_note_owned(self) -> Option<String> {
        match self {
            TokenDestroyFrozenFundsTransition::V0(v0) => v0.public_note_owned(),
        }
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        match self {
            TokenDestroyFrozenFundsTransition::V0(v0) => v0.set_public_note(public_note),
        }
    }

    fn frozen_identity_id(&self) -> Identifier {
        match self {
            TokenDestroyFrozenFundsTransition::V0(v0) => v0.frozen_identity_id(),
        }
    }

    fn set_frozen_identity_id(&mut self, frozen_identity_id: Identifier) {
        match self {
            TokenDestroyFrozenFundsTransition::V0(v0) => {
                v0.set_frozen_identity_id(frozen_identity_id)
            }
        }
    }
}

impl AllowedAsMultiPartyAction for TokenDestroyFrozenFundsTransition {
    fn calculate_action_id(&self, owner_id: Identifier) -> Identifier {
        match self {
            TokenDestroyFrozenFundsTransition::V0(v0) => v0.calculate_action_id(owner_id),
        }
    }
}

impl TokenDestroyFrozenFundsTransition {
    pub fn calculate_action_id_with_fields(
        token_id: &[u8; 32],
        owner_id: &[u8; 32],
        target_id: &[u8; 32],
        identity_contract_nonce: IdentityNonce,
    ) -> Identifier {
        let mut bytes = b"action_token_destroy".to_vec();
        bytes.extend_from_slice(token_id);
        bytes.extend_from_slice(owner_id);
        bytes.extend_from_slice(target_id);
        bytes.extend_from_slice(&identity_contract_nonce.to_be_bytes());

        hash_double(bytes).into()
    }
}
