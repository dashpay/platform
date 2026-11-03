mod v0;

pub use v0::*;

use crate::shielded::SerializedAction;
use crate::state_transition::identity_top_up_from_shielded_pool_transition::IdentityTopUpFromShieldedPoolTransition;
use platform_value::Identifier;

impl IdentityTopUpFromShieldedPoolTransitionAccessorsV0
    for IdentityTopUpFromShieldedPoolTransition
{
    fn identity_id(&self) -> Identifier {
        match self {
            IdentityTopUpFromShieldedPoolTransition::V0(v0) => v0.identity_id,
        }
    }

    fn set_identity_id(&mut self, identity_id: Identifier) {
        match self {
            IdentityTopUpFromShieldedPoolTransition::V0(v0) => v0.identity_id = identity_id,
        }
    }

    fn actions(&self) -> &[SerializedAction] {
        match self {
            IdentityTopUpFromShieldedPoolTransition::V0(v0) => &v0.actions,
        }
    }

    fn set_actions(&mut self, actions: Vec<SerializedAction>) {
        match self {
            IdentityTopUpFromShieldedPoolTransition::V0(v0) => v0.actions = actions,
        }
    }

    fn top_up_amount(&self) -> u64 {
        match self {
            IdentityTopUpFromShieldedPoolTransition::V0(v0) => v0.top_up_amount,
        }
    }

    fn set_top_up_amount(&mut self, top_up_amount: u64) {
        match self {
            IdentityTopUpFromShieldedPoolTransition::V0(v0) => v0.top_up_amount = top_up_amount,
        }
    }

    fn anchor(&self) -> [u8; 32] {
        match self {
            IdentityTopUpFromShieldedPoolTransition::V0(v0) => v0.anchor,
        }
    }

    fn set_anchor(&mut self, anchor: [u8; 32]) {
        match self {
            IdentityTopUpFromShieldedPoolTransition::V0(v0) => v0.anchor = anchor,
        }
    }

    fn proof(&self) -> &[u8] {
        match self {
            IdentityTopUpFromShieldedPoolTransition::V0(v0) => &v0.proof,
        }
    }

    fn set_proof(&mut self, proof: Vec<u8>) {
        match self {
            IdentityTopUpFromShieldedPoolTransition::V0(v0) => v0.proof = proof,
        }
    }

    fn binding_signature(&self) -> [u8; 64] {
        match self {
            IdentityTopUpFromShieldedPoolTransition::V0(v0) => v0.binding_signature,
        }
    }

    fn set_binding_signature(&mut self, binding_signature: [u8; 64]) {
        match self {
            IdentityTopUpFromShieldedPoolTransition::V0(v0) => {
                v0.binding_signature = binding_signature
            }
        }
    }
}
