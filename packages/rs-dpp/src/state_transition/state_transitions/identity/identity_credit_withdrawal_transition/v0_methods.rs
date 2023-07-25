use crate::identity::SecurityLevel;
use crate::prelude::Revision;
use crate::state_transition::identity_credit_withdrawal_transition::v0::v0_methods::IdentityCreditWithdrawalTransitionV0Methods;
use crate::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
use crate::state_transition::{StateTransitionLike, StateTransitionType};
use platform_value::Identifier;

impl IdentityCreditWithdrawalTransitionV0Methods for IdentityCreditWithdrawalTransition {
    fn set_revision(&mut self, revision: Revision) {
        match self {
            IdentityCreditWithdrawalTransition::V0(transition) => transition.set_revision(revision),
        }
    }

    fn revision(&self) -> Revision {
        match self {
            IdentityCreditWithdrawalTransition::V0(transition) => transition.revision(),
        }
    }
}
