mod v0;

use crate::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
use crate::fee::Credits;
use crate::identity::core_script::CoreScript;
use crate::state_transition::address_credit_withdrawal_transition::AddressCreditWithdrawalTransition;
use crate::withdrawal::Pooling;
pub use v0::*;

impl AddressCreditWithdrawalTransitionAccessorsV0 for AddressCreditWithdrawalTransition {
    fn output(&self) -> Option<&(PlatformAddress, Credits)> {
        match self {
            AddressCreditWithdrawalTransition::V0(v0) => v0.output.as_ref(),
        }
    }

    fn set_output(&mut self, output: Option<(PlatformAddress, Credits)>) {
        match self {
            AddressCreditWithdrawalTransition::V0(v0) => v0.output = output,
        }
    }

    fn fee_strategy(&self) -> &AddressFundsFeeStrategy {
        match self {
            AddressCreditWithdrawalTransition::V0(v0) => &v0.fee_strategy,
        }
    }

    fn set_fee_strategy(&mut self, fee_strategy: AddressFundsFeeStrategy) {
        match self {
            AddressCreditWithdrawalTransition::V0(v0) => v0.fee_strategy = fee_strategy,
        }
    }

    fn core_fee_per_byte(&self) -> u32 {
        match self {
            AddressCreditWithdrawalTransition::V0(v0) => v0.core_fee_per_byte,
        }
    }

    fn set_core_fee_per_byte(&mut self, core_fee_per_byte: u32) {
        match self {
            AddressCreditWithdrawalTransition::V0(v0) => v0.core_fee_per_byte = core_fee_per_byte,
        }
    }

    fn pooling(&self) -> Pooling {
        match self {
            AddressCreditWithdrawalTransition::V0(v0) => v0.pooling,
        }
    }

    fn set_pooling(&mut self, pooling: Pooling) {
        match self {
            AddressCreditWithdrawalTransition::V0(v0) => v0.pooling = pooling,
        }
    }

    fn output_script(&self) -> &CoreScript {
        match self {
            AddressCreditWithdrawalTransition::V0(v0) => &v0.output_script,
        }
    }

    fn set_output_script(&mut self, output_script: CoreScript) {
        match self {
            AddressCreditWithdrawalTransition::V0(v0) => v0.output_script = output_script,
        }
    }
}
