mod v0;

use platform_value::Identifier;
pub use v0::*;

use crate::prelude::Revision;
use crate::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
use crate::state_transition::{StateTransitionLike, StateTransitionType};

impl IdentityCreditWithdrawalTransitionAccessorsV0 for IdentityCreditWithdrawalTransition {
    fn identity_id(&self) -> Identifier {
        match self {
            IdentityCreditWithdrawalTransition::V0(transition) => transition.identity_id,
        }
    }

    fn amount(&self) -> u64 {
        match self {
            IdentityCreditWithdrawalTransition::V0(transition) => transition.amount,
        }
    }

    fn set_revision(&mut self, revision: Revision) {
        match self {
            IdentityCreditWithdrawalTransition::V0(transition) => transition.revision = revision,
        }
    }

    fn revision(&self) -> Revision {
        match self {
            IdentityCreditWithdrawalTransition::V0(transition) => transition.revision,
        }
    }
}
