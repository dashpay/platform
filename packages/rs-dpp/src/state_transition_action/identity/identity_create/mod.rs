#[cfg(feature = "state-transition-transformers")]
pub mod transformer;
pub mod v0;

use crate::identity::IdentityPublicKey;
use crate::state_transition_action::identity::identity_create::v0::IdentityCreateTransitionActionV0;
use derive_more::From;
use platform_value::{Bytes36, Identifier};

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
