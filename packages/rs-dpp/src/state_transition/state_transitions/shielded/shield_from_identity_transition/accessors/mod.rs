mod v0;

pub use v0::*;

use crate::prelude::IdentityNonce;
use crate::shielded::SerializedAction;
use crate::state_transition::shield_from_identity_transition::ShieldFromIdentityTransition;
use platform_value::Identifier;

impl ShieldFromIdentityTransitionAccessorsV0 for ShieldFromIdentityTransition {
    fn identity_id(&self) -> Identifier {
        match self {
            ShieldFromIdentityTransition::V0(v0) => v0.identity_id,
        }
    }

    fn set_identity_id(&mut self, identity_id: Identifier) {
        match self {
            ShieldFromIdentityTransition::V0(v0) => v0.identity_id = identity_id,
        }
    }

    fn amount(&self) -> u64 {
        match self {
            ShieldFromIdentityTransition::V0(v0) => v0.amount,
        }
    }

    fn set_amount(&mut self, amount: u64) {
        match self {
            ShieldFromIdentityTransition::V0(v0) => v0.amount = amount,
        }
    }

    fn actions(&self) -> &[SerializedAction] {
        match self {
            ShieldFromIdentityTransition::V0(v0) => &v0.actions,
        }
    }

    fn set_actions(&mut self, actions: Vec<SerializedAction>) {
        match self {
            ShieldFromIdentityTransition::V0(v0) => v0.actions = actions,
        }
    }

    fn anchor(&self) -> [u8; 32] {
        match self {
            ShieldFromIdentityTransition::V0(v0) => v0.anchor,
        }
    }

    fn set_anchor(&mut self, anchor: [u8; 32]) {
        match self {
            ShieldFromIdentityTransition::V0(v0) => v0.anchor = anchor,
        }
    }

    fn proof(&self) -> &[u8] {
        match self {
            ShieldFromIdentityTransition::V0(v0) => &v0.proof,
        }
    }

    fn set_proof(&mut self, proof: Vec<u8>) {
        match self {
            ShieldFromIdentityTransition::V0(v0) => v0.proof = proof,
        }
    }

    fn binding_signature(&self) -> [u8; 64] {
        match self {
            ShieldFromIdentityTransition::V0(v0) => v0.binding_signature,
        }
    }

    fn set_binding_signature(&mut self, binding_signature: [u8; 64]) {
        match self {
            ShieldFromIdentityTransition::V0(v0) => v0.binding_signature = binding_signature,
        }
    }

    fn nonce(&self) -> IdentityNonce {
        match self {
            ShieldFromIdentityTransition::V0(v0) => v0.nonce,
        }
    }

    fn set_nonce(&mut self, nonce: IdentityNonce) {
        match self {
            ShieldFromIdentityTransition::V0(v0) => v0.nonce = nonce,
        }
    }
}
