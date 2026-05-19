mod v0;

pub use v0::*;

use crate::identity::core_script::CoreScript;
use crate::shielded::SerializedAction;
use crate::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition;

impl ShieldedWithdrawalTransitionAccessorsV0 for ShieldedWithdrawalTransition {
    fn actions(&self) -> &[SerializedAction] {
        match self {
            ShieldedWithdrawalTransition::V0(v0) => &v0.actions,
        }
    }

    fn output_script(&self) -> &CoreScript {
        match self {
            ShieldedWithdrawalTransition::V0(v0) => &v0.output_script,
        }
    }
}
