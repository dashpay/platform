mod v0;

use crate::identity::core_script::CoreScript;
use platform_value::Identifier;
pub use v0::*;

use crate::prelude::{IdentityNonce, Revision};
use crate::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;

use crate::withdrawal::Pooling;

impl IdentityCreditWithdrawalTransitionAccessorsV0 for IdentityCreditWithdrawalTransition {
    fn identity_id(&self) -> Identifier {
        match self {
            IdentityCreditWithdrawalTransition::V0(transition) => transition.identity_id,
        }
    }

    fn set_identity_id(&mut self, identity_id: Identifier) {
        match self {
            IdentityCreditWithdrawalTransition::V0(transition) => {
                transition.identity_id = identity_id;
            }
        }
    }

    fn amount(&self) -> u64 {
        match self {
            IdentityCreditWithdrawalTransition::V0(transition) => transition.amount,
        }
    }

    fn set_amount(&mut self, amount: u64) {
        match self {
            IdentityCreditWithdrawalTransition::V0(transition) => {
                transition.amount = amount;
            }
        }
    }

    fn set_nonce(&mut self, nonce: IdentityNonce) {
        match self {
            IdentityCreditWithdrawalTransition::V0(transition) => transition.nonce = nonce,
        }
    }

    fn nonce(&self) -> IdentityNonce {
        match self {
            IdentityCreditWithdrawalTransition::V0(transition) => transition.nonce,
        }
    }

    fn pooling(&self) -> Pooling {
        match self {
            IdentityCreditWithdrawalTransition::V0(transition) => transition.pooling,
        }
    }

    fn set_pooling(&mut self, pooling: Pooling) {
        match self {
            IdentityCreditWithdrawalTransition::V0(transition) => {
                transition.pooling = pooling;
            }
        }
    }

    fn core_fee_per_byte(&self) -> u32 {
        match self {
            IdentityCreditWithdrawalTransition::V0(transition) => transition.core_fee_per_byte,
        }
    }

    fn set_core_fee_per_byte(&mut self, core_fee_per_byte: u32) {
        match self {
            IdentityCreditWithdrawalTransition::V0(transition) => {
                transition.core_fee_per_byte = core_fee_per_byte;
            }
        }
    }

    fn output_script(&self) -> CoreScript {
        match self {
            IdentityCreditWithdrawalTransition::V0(transition) => transition.output_script.clone(),
        }
    }

    fn set_output_script(&mut self, output_script: CoreScript) {
        match self {
            IdentityCreditWithdrawalTransition::V0(transition) => {
                transition.output_script = output_script;
            }
        }
    }
}
