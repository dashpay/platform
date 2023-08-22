pub mod transformer;
pub mod v0;

use crate::state_transition_action::identity::identity_credit_withdrawal::v0::IdentityCreditWithdrawalTransitionActionV0;
use derive_more::From;
use dpp::document::Document;

use dpp::platform_value::Identifier;
use dpp::prelude::Revision;

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

    // Recipient Id
    pub fn prepared_withdrawal_document_owned(self) -> Document {
        match self {
            IdentityCreditWithdrawalTransitionAction::V0(transition) => {
                transition.prepared_withdrawal_document
            }
        }
    }
}
