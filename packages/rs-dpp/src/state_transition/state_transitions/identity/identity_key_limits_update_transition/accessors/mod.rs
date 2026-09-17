mod v0;

use crate::fee::Credits;
use crate::identity::{KeyID, TimestampMillis};
use crate::prelude::{IdentityNonce, Revision};
use crate::state_transition::identity_key_limits_update_transition::IdentityKeyLimitsUpdateTransition;
use platform_value::Identifier;
pub use v0::*;

impl IdentityKeyLimitsUpdateTransitionAccessorsV0 for IdentityKeyLimitsUpdateTransition {
    fn set_identity_id(&mut self, id: Identifier) {
        match self {
            IdentityKeyLimitsUpdateTransition::V0(transition) => transition.set_identity_id(id),
        }
    }

    fn identity_id(&self) -> Identifier {
        match self {
            IdentityKeyLimitsUpdateTransition::V0(transition) => transition.identity_id(),
        }
    }

    fn set_revision(&mut self, revision: Revision) {
        match self {
            IdentityKeyLimitsUpdateTransition::V0(transition) => transition.set_revision(revision),
        }
    }

    fn revision(&self) -> Revision {
        match self {
            IdentityKeyLimitsUpdateTransition::V0(transition) => transition.revision(),
        }
    }

    fn set_nonce(&mut self, nonce: IdentityNonce) {
        match self {
            IdentityKeyLimitsUpdateTransition::V0(transition) => transition.set_nonce(nonce),
        }
    }

    fn nonce(&self) -> IdentityNonce {
        match self {
            IdentityKeyLimitsUpdateTransition::V0(transition) => transition.nonce(),
        }
    }

    fn set_key_id(&mut self, key_id: KeyID) {
        match self {
            IdentityKeyLimitsUpdateTransition::V0(transition) => transition.set_key_id(key_id),
        }
    }

    fn key_id(&self) -> KeyID {
        match self {
            IdentityKeyLimitsUpdateTransition::V0(transition) => transition.key_id(),
        }
    }

    fn set_total_budget(&mut self, total_budget: Option<Credits>) {
        match self {
            IdentityKeyLimitsUpdateTransition::V0(transition) => {
                transition.set_total_budget(total_budget)
            }
        }
    }

    fn total_budget(&self) -> Option<Credits> {
        match self {
            IdentityKeyLimitsUpdateTransition::V0(transition) => transition.total_budget(),
        }
    }

    fn set_expires_at(&mut self, expires_at: Option<TimestampMillis>) {
        match self {
            IdentityKeyLimitsUpdateTransition::V0(transition) => {
                transition.set_expires_at(expires_at)
            }
        }
    }

    fn expires_at(&self) -> Option<TimestampMillis> {
        match self {
            IdentityKeyLimitsUpdateTransition::V0(transition) => transition.expires_at(),
        }
    }
}
