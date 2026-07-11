/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::shielded::identity_create_from_shielded_pool::v0::IdentityCreateFromShieldedPoolTransitionActionV0;
use crate::state_transition_action::shielded::ShieldedActionNote;
use derive_more::From;
use dpp::fee::Credits;
use dpp::identity::Identity;
use dpp::prelude::Identifier;

/// IdentityCreateFromShieldedPool transition action
#[derive(Debug, Clone, From)]
pub enum IdentityCreateFromShieldedPoolTransitionAction {
    /// v0
    V0(IdentityCreateFromShieldedPoolTransitionActionV0),
}

impl IdentityCreateFromShieldedPoolTransitionAction {
    /// Get the built identity (balance = denomination, before fee deduction).
    pub fn identity(&self) -> &Identity {
        match self {
            IdentityCreateFromShieldedPoolTransitionAction::V0(transition) => &transition.identity,
        }
    }
    /// Take ownership of the built identity.
    pub fn identity_owned(self) -> Identity {
        match self {
            IdentityCreateFromShieldedPoolTransitionAction::V0(transition) => transition.identity,
        }
    }
    /// Get the id of the new identity.
    pub fn identity_id(&self) -> Identifier {
        use dpp::identity::accessors::IdentityGettersV0;
        self.identity().id()
    }
    /// Get notes.
    pub fn notes(&self) -> &[ShieldedActionNote] {
        match self {
            IdentityCreateFromShieldedPoolTransitionAction::V0(transition) => &transition.notes,
        }
    }
    /// Get anchor.
    pub fn anchor(&self) -> &[u8; 32] {
        match self {
            IdentityCreateFromShieldedPoolTransitionAction::V0(transition) => &transition.anchor,
        }
    }
    /// Get the exit denomination (in credits).
    pub fn denomination(&self) -> Credits {
        match self {
            IdentityCreateFromShieldedPoolTransitionAction::V0(transition) => {
                transition.denomination
            }
        }
    }
    /// Total fee moved from the new identity's balance into the fee pools at execution.
    pub fn fee_amount(&self) -> Credits {
        match self {
            IdentityCreateFromShieldedPoolTransitionAction::V0(transition) => transition.fee_amount,
        }
    }
    /// Current total balance of the shielded pool (before this transition).
    pub fn current_total_balance(&self) -> Credits {
        match self {
            IdentityCreateFromShieldedPoolTransitionAction::V0(transition) => {
                transition.current_total_balance
            }
        }
    }
}
