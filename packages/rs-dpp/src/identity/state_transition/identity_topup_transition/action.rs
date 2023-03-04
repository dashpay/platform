use serde::{Deserialize, Serialize};
use crate::identifier::Identifier;
use crate::identity::state_transition::identity_topup_transition::IdentityTopUpTransition;
use crate::identity::state_transition::identity_public_key_transitions::IdentityPublicKeyTopUpTransition;

pub const IDENTITY_TOP_UP_TRANSITION_ACTION_VERSION: u32 = 0;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityTopUpTransitionAction {
    pub version: u32,
    pub top_up_balance_amount: u64,
    pub identity_id: Identifier,
}

impl IdentityTopUpTransitionAction {
    fn from(value: IdentityTopUpTransition, top_up_balance_amount: u64) -> Self {
        let IdentityTopUpTransition {
            identity_id, ..
        } = value;
        IdentityTopUpTransitionAction {
            version: IDENTITY_TOP_UP_TRANSITION_ACTION_VERSION,
            top_up_balance_amount,
            identity_id,
        }
    }
}