mod v0;

use crate::fee::Credits;
use crate::identity::KeyOfType;
use crate::prelude::KeyOfTypeNonce;
use crate::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition;
use std::collections::BTreeMap;
pub use v0::*;

impl AddressFundsTransferTransitionAccessorsV0 for AddressFundsTransferTransition {
    fn inputs(&self) -> &BTreeMap<KeyOfType, (KeyOfTypeNonce, Credits)> {
        match self {
            AddressFundsTransferTransition::V0(transition) => &transition.inputs,
        }
    }

    fn set_inputs(&mut self, inputs: BTreeMap<KeyOfType, (KeyOfTypeNonce, Credits)>) {
        match self {
            AddressFundsTransferTransition::V0(transition) => {
                transition.inputs = inputs;
            }
        }
    }

    fn outputs(&self) -> &BTreeMap<KeyOfType, Credits> {
        match self {
            AddressFundsTransferTransition::V0(transition) => &transition.outputs,
        }
    }

    fn set_outputs(&mut self, outputs: BTreeMap<KeyOfType, Credits>) {
        match self {
            AddressFundsTransferTransition::V0(transition) => {
                transition.outputs = outputs;
            }
        }
    }
}
