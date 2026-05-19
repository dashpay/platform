mod v0;

pub use v0::*;

use crate::shielded::SerializedAction;
use crate::state_transition::shielded_transfer_transition::ShieldedTransferTransition;

impl ShieldedTransferTransitionAccessorsV0 for ShieldedTransferTransition {
    fn actions(&self) -> &[SerializedAction] {
        match self {
            ShieldedTransferTransition::V0(v0) => &v0.actions,
        }
    }
}
