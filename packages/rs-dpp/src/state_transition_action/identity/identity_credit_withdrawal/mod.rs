pub mod v0;
#[cfg(feature = "state-transition-transformers")]
pub mod transformer;

use crate::document::Document;
use crate::fee::Credits;
use crate::prelude::Revision;
use crate::state_transition::identity_credit_withdrawal_transition::v0_action::IdentityCreditWithdrawalTransitionActionV0;
use derive_more::From;
use platform_value::{Bytes36, Identifier};
use crate::state_transition_action::identity::identity_credit_withdrawal::v0::IdentityCreditWithdrawalTransitionActionV0;

#[derive(Debug, Clone, From)]
pub enum IdentityCreditWithdrawalTransitionAction {
    V0(IdentityCreditWithdrawalTransitionActionV0),
}

impl IdentityCreditWithdrawalTransitionAction {
    // Withdrawal amount
    pub fn revision(&self) -> Revision {
        match self {
            IdentityCreditWithdrawalTransitionAction::V0(transition) => transition.revision,
        }
    }

    // Identity Id
    pub fn identity_id(&self) -> Identifier {
        match self {
            IdentityCreditWithdrawalTransitionAction::V0(transition) => transition.identity_id,
        }
    }

    // Recipient Id
    pub fn prepared_withdrawal_document(&self) -> &Document {
        match self {
            IdentityCreditWithdrawalTransitionAction::V0(transition) => {
                &transition.prepared_withdrawal_document
            }
        }
    }
}
