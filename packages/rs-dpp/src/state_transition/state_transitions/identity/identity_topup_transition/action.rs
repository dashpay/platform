use derive_more::From;
use platform_value::{Bytes36, Identifier};
use crate::identity::IdentityPublicKey;
use crate::state_transition::identity_topup_transition::v0_action::IdentityTopUpTransitionActionV0;

#[derive(Debug, Clone, From)]
pub enum IdentityTopUpTransitionAction {
    V0(IdentityTopUpTransitionActionV0),
}

impl IdentityTopUpTransitionAction {
    // The balance being topped up
    pub fn top_up_balance_amount(&self) -> u64 {
        match self {
            IdentityTopUpTransitionAction::V0(transition) => transition.top_up_balance_amount,
        }
    }


    // Identity Id
    pub fn identity_id(&self) -> Identifier {
        match self {
            IdentityTopUpTransitionAction::V0(transition) => transition.identity_id,
        }
    }

    // Asset Lock Outpoint
    pub fn asset_lock_outpoint(&self) -> Bytes36 {
        match self {
            IdentityTopUpTransitionAction::V0(transition) => transition.asset_lock_outpoint,
        }
    }
}