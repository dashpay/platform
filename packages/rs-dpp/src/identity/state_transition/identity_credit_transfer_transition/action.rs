use crate::identity::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
use crate::state_transition::fee::Credits;
use platform_value::Identifier;
use serde::{Deserialize, Serialize};

pub const IDENTITY_CREDIT_TRANSFER_TRANSITION_ACTION_VERSION: u32 = 0;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityCreditTransferTransitionAction {
    pub version: u32,
    pub transfer_amount: Credits,
    pub recipient_id: Identifier,
    pub identity_id: Identifier,
}

impl From<IdentityCreditTransferTransition> for IdentityCreditTransferTransitionAction {
    fn from(value: IdentityCreditTransferTransition) -> Self {
        let IdentityCreditTransferTransition {
            identity_id: owner_id,
            recipient_id,
            amount,
            ..
        } = value;
        IdentityCreditTransferTransitionAction {
            version: IDENTITY_CREDIT_TRANSFER_TRANSITION_ACTION_VERSION,
            identity_id: owner_id,
            recipient_id,
            transfer_amount: amount,
        }
    }
}

impl From<&IdentityCreditTransferTransition> for IdentityCreditTransferTransitionAction {
    fn from(value: &IdentityCreditTransferTransition) -> Self {
        let IdentityCreditTransferTransition {
            identity_id,
            recipient_id,
            amount,
            ..
        } = value;
        IdentityCreditTransferTransitionAction {
            version: IDENTITY_CREDIT_TRANSFER_TRANSITION_ACTION_VERSION,
            identity_id: *identity_id,
            recipient_id: *recipient_id,
            transfer_amount: *amount,
        }
    }
}
