use crate::prelude::Identity;
use derive_more::From;
use platform_value::{Bytes36, Identifier};
use crate::identity::IdentityPublicKey;
use crate::state_transition::identity_create_transition::IdentityCreateTransition;
use crate::state_transition::identity_create_transition::v0_action::IdentityCreateTransitionActionV0;

#[derive(Debug, Clone, From)]
pub enum IdentityCreateTransitionAction {
    V0(IdentityCreateTransitionActionV0),
}

impl IdentityCreateTransitionAction {
    // Public Keys
    pub fn public_keys(&self) -> &Vec<IdentityPublicKey> {
        match self {
            IdentityCreateTransitionAction::V0(transition) => &transition.public_keys,
        }
    }

    // Initial Balance Amount
    pub fn initial_balance_amount(&self) -> u64 {
        match self {
            IdentityCreateTransitionAction::V0(transition) => transition.initial_balance_amount,
        }
    }

    // Identity Id
    pub fn identity_id(&self) -> Identifier {
        match self {
            IdentityCreateTransitionAction::V0(transition) => transition.identity_id,
        }
    }

    // Asset Lock Outpoint
    pub fn asset_lock_outpoint(&self) -> Bytes36 {
        match self {
            IdentityCreateTransitionAction::V0(transition) => transition.asset_lock_outpoint,
        }
    }
}