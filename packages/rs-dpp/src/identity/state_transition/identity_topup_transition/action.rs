use crate::identifier::Identifier;
use crate::identity::state_transition::identity_topup_transition::IdentityTopUpTransition;
use serde::{Deserialize, Serialize};

pub const IDENTITY_TOP_UP_TRANSITION_ACTION_VERSION: u32 = 0;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityTopUpTransitionAction {
    pub version: u32,
    pub top_up_balance_amount: u64,
    pub identity_id: Identifier,
}

impl IdentityTopUpTransitionAction {
    pub fn from(value: IdentityTopUpTransition, top_up_balance_amount: u64) -> Self {
        let IdentityTopUpTransition { identity_id, .. } = value;
        IdentityTopUpTransitionAction {
            version: IDENTITY_TOP_UP_TRANSITION_ACTION_VERSION,
            top_up_balance_amount,
            identity_id,
        }
    }

    pub fn from_borrowed(value: &IdentityTopUpTransition, top_up_balance_amount: u64) -> Self {
        let IdentityTopUpTransition { identity_id, .. } = value;
        IdentityTopUpTransitionAction {
            version: IDENTITY_TOP_UP_TRANSITION_ACTION_VERSION,
            top_up_balance_amount,
            identity_id: *identity_id,
        }
    }
}
