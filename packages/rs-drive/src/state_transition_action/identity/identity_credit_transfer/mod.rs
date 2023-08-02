pub mod transformer;
pub mod v0;

use crate::state_transition_action::identity::identity_credit_transfer::v0::IdentityCreditTransferTransitionActionV0;
use derive_more::From;
use dpp::fee::Credits;
use dpp::platform_value::{Bytes36, Identifier};

#[derive(Debug, Clone, From)]
pub enum IdentityCreditTransferTransitionAction {
    V0(IdentityCreditTransferTransitionActionV0),
}

impl IdentityCreditTransferTransitionAction {
    // Transfer amount
    pub fn transfer_amount(&self) -> Credits {
        match self {
            IdentityCreditTransferTransitionAction::V0(transition) => transition.transfer_amount,
        }
    }

    // Identity Id
    pub fn identity_id(&self) -> Identifier {
        match self {
            IdentityCreditTransferTransitionAction::V0(transition) => transition.identity_id,
        }
    }

    // Recipient Id
    pub fn recipient_id(&self) -> Identifier {
        match self {
            IdentityCreditTransferTransitionAction::V0(transition) => transition.recipient_id,
        }
    }
}
