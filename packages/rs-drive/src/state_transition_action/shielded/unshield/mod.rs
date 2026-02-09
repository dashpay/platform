/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::shielded::unshield::v0::UnshieldTransitionActionV0;
use derive_more::From;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::prelude::UserFeeIncrease;

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
    /// Get nullifiers
    pub fn nullifiers(&self) -> &Vec<[u8; 32]> {
        match self {
            UnshieldTransitionAction::V0(transition) => &transition.nullifiers,
        }
    }
    /// Get note commitments
    pub fn note_commitments(&self) -> &Vec<[u8; 32]> {
        match self {
            UnshieldTransitionAction::V0(transition) => &transition.note_commitments,
        }
    }
    /// Get encrypted notes
    pub fn encrypted_notes(&self) -> &Vec<Vec<u8>> {
        match self {
            UnshieldTransitionAction::V0(transition) => &transition.encrypted_notes,
        }
    }
    /// Get anchor
    pub fn anchor(&self) -> &[u8; 32] {
        match self {
            UnshieldTransitionAction::V0(transition) => &transition.anchor,
        }
    }
    /// fee multiplier
    pub fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            UnshieldTransitionAction::V0(transition) => transition.user_fee_increase,
        }
    }
}
