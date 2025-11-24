use derive_more::From;
use dpp::fee::Credits;
use dpp::identity::KeyOfType;
use std::collections::BTreeMap;

use dpp::prelude::{KeyOfTypeNonce, UserFeeIncrease};

/// transformer module
pub mod transformer;
mod v0;

use crate::state_transition_action::address_funds::address_funds_transfer::AddressFundsTransferTransitionAction;
pub use v0::*;

/// bump address_input nonce action
#[derive(Debug, Clone, From)]
pub enum BumpAddressInputNonceAction {
    /// v0
    V0(BumpAddressInputNonceActionV0),
}

impl BumpAddressInputNonceActionAccessorsV0 for BumpAddressInputNonceAction {
    /// Get inputs
    fn inputs_with_remaining_balance(&self) -> &BTreeMap<KeyOfType, (KeyOfTypeNonce, Credits)> {
        match self {
            AddressFundsTransferTransitionAction::V0(transition) => {
                &transition.inputs_with_remaining_balance
            }
        }
    }
    /// Returns owned copies of inputs and outputs.
    fn inputs_with_remaining_balance_and_outputs_owned(
        self,
    ) -> (
        BTreeMap<KeyOfType, (KeyOfTypeNonce, Credits)>,
        BTreeMap<KeyOfType, Credits>,
    ) {
        match self {
            AddressFundsTransferTransitionAction::V0(transition) => {
                (transition.inputs_with_remaining_balance, transition.outputs)
            }
        }
    }

    fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            BumpAddressInputNonceAction::V0(transition) => transition.user_fee_increase,
        }
    }
}
