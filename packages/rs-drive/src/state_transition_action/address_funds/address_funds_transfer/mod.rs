/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::address_funds::address_funds_transfer::v0::AddressFundsTransferTransitionActionV0;
use derive_more::From;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, UserFeeIncrease};
use std::collections::BTreeMap;

/// action
#[derive(Debug, Clone, From)]
pub enum AddressFundsTransferTransitionAction {
    /// v0
    V0(AddressFundsTransferTransitionActionV0),
}

impl AddressFundsTransferTransitionAction {
    /// Get inputs
    pub fn inputs_with_remaining_balance(&self) -> &BTreeMap<PlatformAddress, (AddressNonce, Credits)> {
        match self {
            AddressFundsTransferTransitionAction::V0(transition) => {
                &transition.inputs_with_remaining_balance
            }
        }
    }
    /// Get outputs
    pub fn outputs(&self) -> &BTreeMap<PlatformAddress, Credits> {
        match self {
            AddressFundsTransferTransitionAction::V0(transition) => &transition.outputs,
        }
    }
    /// Returns owned copies of inputs and outputs.
    pub fn inputs_with_remaining_balance_and_outputs_owned(
        self,
    ) -> (
        BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        BTreeMap<PlatformAddress, Credits>,
    ) {
        match self {
            AddressFundsTransferTransitionAction::V0(transition) => {
                (transition.inputs_with_remaining_balance, transition.outputs)
            }
        }
    }
    /// fee multiplier
    pub fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            AddressFundsTransferTransitionAction::V0(transition) => transition.user_fee_increase,
        }
    }
}
