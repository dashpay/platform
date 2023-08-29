/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::identity::identity_topup::v0::IdentityTopUpTransitionActionV0;
use derive_more::From;

use dpp::platform_value::{Bytes36, Identifier};

/// action
#[derive(Debug, Clone, From)]
pub enum IdentityTopUpTransitionAction {
    /// v0
    V0(IdentityTopUpTransitionActionV0),
}

impl IdentityTopUpTransitionAction {
    /// The balance being topped up
    pub fn top_up_balance_amount(&self) -> u64 {
        match self {
            IdentityTopUpTransitionAction::V0(transition) => transition.top_up_balance_amount,
        }
    }

    /// Identity Id
    pub fn identity_id(&self) -> Identifier {
        match self {
            IdentityTopUpTransitionAction::V0(transition) => transition.identity_id,
        }
    }

    /// Asset Lock Outpoint
    pub fn asset_lock_outpoint(&self) -> Bytes36 {
        match self {
            IdentityTopUpTransitionAction::V0(transition) => transition.asset_lock_outpoint,
        }
    }
}
