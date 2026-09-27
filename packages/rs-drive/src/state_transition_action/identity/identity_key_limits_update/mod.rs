/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::identity::identity_key_limits_update::v0::IdentityKeyLimitsUpdateTransitionActionV0;
use derive_more::From;
use dpp::fee::Credits;
use dpp::identity::{IdentityPublicKey, KeyID, TimestampMillis};
use dpp::platform_value::Identifier;
use dpp::prelude::{IdentityNonce, UserFeeIncrease};

/// The action of an identity key limits update: raises the total budget of one of the
/// identity's keys, and the remaining budget with it, or moves its expiry later.
#[derive(Debug, Clone, From)]
pub enum IdentityKeyLimitsUpdateTransitionAction {
    /// v0
    V0(IdentityKeyLimitsUpdateTransitionActionV0),
}

impl IdentityKeyLimitsUpdateTransitionAction {
    /// Identity Id
    pub fn identity_id(&self) -> Identifier {
        match self {
            IdentityKeyLimitsUpdateTransitionAction::V0(transition) => transition.identity_id,
        }
    }

    /// Nonce
    pub fn nonce(&self) -> IdentityNonce {
        match self {
            IdentityKeyLimitsUpdateTransitionAction::V0(transition) => transition.nonce,
        }
    }

    /// The key whose limits are raised
    pub fn key_id(&self) -> KeyID {
        match self {
            IdentityKeyLimitsUpdateTransitionAction::V0(transition) => transition.key_id,
        }
    }

    /// The key as stored before the update
    pub fn stored_key(&self) -> &IdentityPublicKey {
        match self {
            IdentityKeyLimitsUpdateTransitionAction::V0(transition) => &transition.stored_key,
        }
    }

    /// The new total budget of the key, `None` when it stays as it is
    pub fn total_budget(&self) -> Option<Credits> {
        match self {
            IdentityKeyLimitsUpdateTransitionAction::V0(transition) => transition.total_budget,
        }
    }

    /// The new expiry of the key, `None` when it stays as it is
    pub fn expires_at(&self) -> Option<TimestampMillis> {
        match self {
            IdentityKeyLimitsUpdateTransitionAction::V0(transition) => transition.expires_at,
        }
    }

    /// fee multiplier
    pub fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            IdentityKeyLimitsUpdateTransitionAction::V0(transition) => transition.user_fee_increase,
        }
    }
}
