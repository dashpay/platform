/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::shielded::unshield::v0::UnshieldTransitionActionV0;
use crate::state_transition_action::shielded::ShieldedActionNote;
use derive_more::From;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;

/// Unshield transition action
#[derive(Debug, Clone, From)]
pub enum UnshieldTransitionAction {
    /// v0
    V0(UnshieldTransitionActionV0),
}

impl UnshieldTransitionAction {
    /// Get output address
    pub fn output_address(&self) -> &PlatformAddress {
        match self {
            UnshieldTransitionAction::V0(transition) => &transition.output_address,
        }
    }
    /// Get amount
    pub fn amount(&self) -> Credits {
        match self {
            UnshieldTransitionAction::V0(transition) => transition.amount,
        }
    }
    /// Get notes
    pub fn notes(&self) -> &[ShieldedActionNote] {
        match self {
            UnshieldTransitionAction::V0(transition) => &transition.notes,
        }
    }
    /// Get anchor
    pub fn anchor(&self) -> &[u8; 32] {
        match self {
            UnshieldTransitionAction::V0(transition) => &transition.anchor,
        }
    }
    /// Fee amount (value_balance - amount), paid to proposers
    pub fn fee_amount(&self) -> Credits {
        match self {
            UnshieldTransitionAction::V0(transition) => transition.fee_amount,
        }
    }
}
