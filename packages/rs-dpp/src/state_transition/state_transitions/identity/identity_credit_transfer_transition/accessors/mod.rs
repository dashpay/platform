mod v0;

use crate::prelude::Revision;
use crate::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
use platform_value::Identifier;
pub use v0::*;

impl IdentityCreditTransferTransitionAccessorsV0 for IdentityCreditTransferTransition {
    fn set_amount(&mut self, amount: u64) {
        match self {
            IdentityCreditTransferTransition::V0(transition) => {
                transition.amount = amount;
            }
        }
    }

    fn amount(&self) -> u64 {
        match self {
            IdentityCreditTransferTransition::V0(transition) => transition.amount,
        }
    }

    fn identity_id(&self) -> Identifier {
        match self {
            IdentityCreditTransferTransition::V0(transition) => transition.identity_id,
        }
    }

    fn set_identity_id(&mut self, identity_id: Identifier) {
        match self {
            IdentityCreditTransferTransition::V0(transition) => {
                transition.identity_id = identity_id;
            }
        }
    }

    fn recipient_id(&self) -> Identifier {
        match self {
            IdentityCreditTransferTransition::V0(transition) => transition.recipient_id,
        }
    }

    fn set_recipient_id(&mut self, recipient_id: Identifier) {
        match self {
            IdentityCreditTransferTransition::V0(transition) => {
                transition.recipient_id = recipient_id;
            }
        }
    }

    fn set_revision(&mut self, revision: Revision) {
        match self {
            IdentityCreditTransferTransition::V0(transition) => transition.revision = revision,
        }
    }

    fn revision(&self) -> Revision {
        match self {
            IdentityCreditTransferTransition::V0(transition) => transition.revision,
        }
    }
}
