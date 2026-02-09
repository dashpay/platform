/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::shielded::shielded_transfer::v0::ShieldedTransferTransitionActionV0;
use derive_more::From;
use dpp::fee::Credits;
use dpp::prelude::UserFeeIncrease;

/// Shielded transfer transition action
#[derive(Debug, Clone, From)]
pub enum ShieldedTransferTransitionAction {
    /// v0
    V0(ShieldedTransferTransitionActionV0),
}

impl ShieldedTransferTransitionAction {
    /// Get nullifiers
    pub fn nullifiers(&self) -> &Vec<[u8; 32]> {
        match self {
            ShieldedTransferTransitionAction::V0(transition) => &transition.nullifiers,
        }
    }
    /// Get note commitments
    pub fn note_commitments(&self) -> &Vec<[u8; 32]> {
        match self {
            ShieldedTransferTransitionAction::V0(transition) => &transition.note_commitments,
        }
    }
    /// Get encrypted notes
    pub fn encrypted_notes(&self) -> &Vec<Vec<u8>> {
        match self {
            ShieldedTransferTransitionAction::V0(transition) => &transition.encrypted_notes,
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
    /// fee multiplier
    pub fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            ShieldedTransferTransitionAction::V0(transition) => transition.user_fee_increase,
        }
    }
}
