/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::shielded::identity_top_up_from_shielded_pool::v0::IdentityTopUpFromShieldedPoolTransitionActionV0;
use crate::state_transition_action::shielded::ShieldedActionNote;
use derive_more::From;
use dpp::fee::Credits;
use dpp::platform_value::Identifier;

/// Identity top up from shielded pool transition action
#[derive(Debug, Clone, From)]
pub enum IdentityTopUpFromShieldedPoolTransitionAction {
    /// v0
    V0(IdentityTopUpFromShieldedPoolTransitionActionV0),
}

impl IdentityTopUpFromShieldedPoolTransitionAction {
    /// The identity whose balance is credited
    pub fn identity_id(&self) -> Identifier {
        match self {
            IdentityTopUpFromShieldedPoolTransitionAction::V0(t) => t.identity_id,
        }
    }
    /// Gross amount leaving the pool
    pub fn amount(&self) -> Credits {
        match self {
            IdentityTopUpFromShieldedPoolTransitionAction::V0(t) => t.amount,
        }
    }
    /// Notes (spent nullifiers plus change outputs)
    pub fn notes(&self) -> &[ShieldedActionNote] {
        match self {
            IdentityTopUpFromShieldedPoolTransitionAction::V0(t) => &t.notes,
        }
    }
    /// The anchor the spend was proven against
    pub fn anchor(&self) -> &[u8; 32] {
        match self {
            IdentityTopUpFromShieldedPoolTransitionAction::V0(t) => &t.anchor,
        }
    }
    /// Flat fee routed to the fee pools
    pub fn fee_amount(&self) -> Credits {
        match self {
            IdentityTopUpFromShieldedPoolTransitionAction::V0(t) => t.fee_amount,
        }
    }
    /// Pool total balance read at transform time
    pub fn current_total_balance(&self) -> Credits {
        match self {
            IdentityTopUpFromShieldedPoolTransitionAction::V0(t) => t.current_total_balance,
        }
    }
}
