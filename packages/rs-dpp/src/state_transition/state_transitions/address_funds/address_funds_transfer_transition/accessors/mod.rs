mod v0;

use std::collections::BTreeMap;

use crate::address_funds::PlatformAddress;
use crate::fee::Credits;
use crate::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition;
pub use v0::*;

impl AddressFundsTransferTransitionAccessorsV0 for AddressFundsTransferTransition {
    fn outputs(&self) -> &BTreeMap<PlatformAddress, Credits> {
        match self {
            AddressFundsTransferTransition::V0(transition) => &transition.outputs,
        }
    }

    fn set_outputs(&mut self, outputs: BTreeMap<PlatformAddress, Credits>) {
        match self {
            AddressFundsTransferTransition::V0(transition) => {
                transition.outputs = outputs;
            }
        }
    }
}
