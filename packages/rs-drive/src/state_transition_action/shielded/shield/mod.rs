/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::shielded::shield::v0::ShieldTransitionActionV0;
use crate::state_transition_action::shielded::ShieldedActionNote;
use derive_more::From;
use dpp::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
use dpp::fee::Credits;
use dpp::prelude::{AddressNonce, UserFeeIncrease};
use std::collections::BTreeMap;

/// Shield transition action
#[derive(Debug, Clone, From)]
pub enum ShieldTransitionAction {
    /// v0
    V0(ShieldTransitionActionV0),
}

impl ShieldTransitionAction {
    /// Get inputs with remaining balance
    pub fn inputs_with_remaining_balance(
        &self,
    ) -> &BTreeMap<PlatformAddress, (AddressNonce, Credits)> {
        match self {
            ShieldTransitionAction::V0(transition) => &transition.inputs_with_remaining_balance,
        }
    }
    /// Get the shield amount
    pub fn shield_amount(&self) -> Credits {
        match self {
            ShieldTransitionAction::V0(transition) => transition.shield_amount,
        }
    }
    /// Get notes
    pub fn notes(&self) -> &[ShieldedActionNote] {
        match self {
            ShieldTransitionAction::V0(transition) => &transition.notes,
        }
    }
    /// fee multiplier
    pub fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            ShieldTransitionAction::V0(transition) => transition.user_fee_increase,
        }
    }
    /// fee strategy
    pub fn fee_strategy(&self) -> &AddressFundsFeeStrategy {
        match self {
            ShieldTransitionAction::V0(transition) => &transition.fee_strategy,
        }
    }
}
