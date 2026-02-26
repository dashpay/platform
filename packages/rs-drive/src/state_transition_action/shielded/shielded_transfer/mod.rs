/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::shielded::shielded_transfer::v0::ShieldedTransferTransitionActionV0;
use crate::state_transition_action::shielded::ShieldedActionNote;
use derive_more::From;
use dpp::fee::Credits;

/// Shielded transfer transition action
#[derive(Debug, Clone, From)]
pub enum ShieldedTransferTransitionAction {
    /// v0
    V0(ShieldedTransferTransitionActionV0),
}

impl ShieldedTransferTransitionAction {
    /// Get notes
    pub fn notes(&self) -> &[ShieldedActionNote] {
        match self {
            ShieldedTransferTransitionAction::V0(transition) => &transition.notes,
        }
    }
    /// Get anchor
    pub fn anchor(&self) -> &[u8; 32] {
        match self {
            ShieldedTransferTransitionAction::V0(transition) => &transition.anchor,
        }
    }
    /// Get fee amount
    pub fn fee_amount(&self) -> Credits {
        match self {
            ShieldedTransferTransitionAction::V0(transition) => transition.fee_amount,
        }
    }
}
