mod v0;

pub use v0::*;

use crate::prelude::AssetLockProof;
use crate::state_transition::identity_topup_transition::IdentityTopUpTransition;

use platform_value::Identifier;

impl IdentityTopUpTransitionAccessorsV0 for IdentityTopUpTransition {
    fn set_identity_id(&mut self, identity_id: Identifier) {
        match self {
            IdentityTopUpTransition::V0(transition) => transition.set_identity_id(identity_id),
        }
    }

    fn identity_id(&self) -> &Identifier {
        match self {
            IdentityTopUpTransition::V0(transition) => transition.identity_id(),
        }
    }
}
