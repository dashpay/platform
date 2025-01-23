use platform_value::Identifier;
use crate::balances::credits::TokenAmount;
use crate::prelude::IdentityNonce;
use crate::state_transition::batch_transition::batched_transition::multi_party_action::AllowedAsMultiPartyAction;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
use crate::state_transition::batch_transition::token_burn_transition::TokenBurnTransition;
use crate::state_transition::batch_transition::token_burn_transition::v0::v0_methods::TokenBurnTransitionV0Methods;
use crate::util::hash::hash_double;

impl TokenBaseTransitionAccessors for TokenBurnTransition {
    fn base(&self) -> &TokenBaseTransition {
        match self {
            TokenBurnTransition::V0(v0) => &v0.base,
        }
    }

    fn base_mut(&mut self) -> &mut TokenBaseTransition {
        match self {
            TokenBurnTransition::V0(v0) => &mut v0.base,
        }
    }

    fn set_base(&mut self, base: TokenBaseTransition) {
        match self {
            TokenBurnTransition::V0(v0) => v0.base = base,
        }
    }
}

impl TokenBurnTransitionV0Methods for TokenBurnTransition {
    fn burn_amount(&self) -> u64 {
        match self {
            TokenBurnTransition::V0(v0) => v0.burn_amount(),
        }
    }

    fn set_burn_amount(&mut self, burn_amount: u64) {
        match self {
            TokenBurnTransition::V0(v0) => v0.set_burn_amount(burn_amount),
        }
    }

    fn public_note(&self) -> Option<&String> {
        match self {
            TokenBurnTransition::V0(v0) => v0.public_note(),
        }
    }

    fn public_note_owned(self) -> Option<String> {
        match self {
            TokenBurnTransition::V0(v0) => v0.public_note_owned(),
        }
    }

    fn set_public_note(&mut self, public_note: Option<String>) {
        match self {
            TokenBurnTransition::V0(v0) => v0.set_public_note(public_note),
        }
    }
}

impl AllowedAsMultiPartyAction for TokenBurnTransition {
    fn calculate_action_id(&self, owner_id: Identifier) -> Identifier {
        match self {
            TokenBurnTransition::V0(v0) => v0.calculate_action_id(owner_id),
        }
    }
}

impl TokenBurnTransition {
    pub fn calculate_action_id_with_fields(
        token_id: &[u8; 32],
        owner_id: &[u8; 32],
        identity_contract_nonce: IdentityNonce,
        burn_amount: TokenAmount,
    ) -> Identifier {
        let mut bytes = b"action_token_burn".to_vec();
        bytes.extend_from_slice(token_id);
        bytes.extend_from_slice(owner_id);
        bytes.extend_from_slice(&identity_contract_nonce.to_be_bytes());
        bytes.extend_from_slice(&burn_amount.to_be_bytes());

        hash_double(bytes).into()
    }
}
