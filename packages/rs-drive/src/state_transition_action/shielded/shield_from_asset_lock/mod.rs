/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::shielded::shield_from_asset_lock::v0::ShieldFromAssetLockTransitionActionV0;
use derive_more::From;
use dpp::fee::Credits;
use dpp::prelude::UserFeeIncrease;

/// Shield from asset lock transition action
#[derive(Debug, Clone, From)]
pub enum ShieldFromAssetLockTransitionAction {
    /// v0
    V0(ShieldFromAssetLockTransitionActionV0),
}

impl ShieldFromAssetLockTransitionAction {
    /// Get asset lock outpoint
    pub fn asset_lock_outpoint(&self) -> &[u8; 36] {
        match self {
            ShieldFromAssetLockTransitionAction::V0(transition) => {
                &transition.asset_lock_outpoint
            }
        }
    }
    /// Get remaining asset lock value to be consumed
    pub fn asset_lock_value_to_be_consumed(&self) -> Credits {
        match self {
            ShieldFromAssetLockTransitionAction::V0(transition) => {
                transition.asset_lock_value_to_be_consumed
            }
        }
    }
    /// Get signable bytes hasher
    pub fn signable_bytes_hasher(&self) -> &[u8; 32] {
        match self {
            ShieldFromAssetLockTransitionAction::V0(transition) => {
                &transition.signable_bytes_hasher
            }
        }
    }
    /// Get the shield amount
    pub fn shield_amount(&self) -> Credits {
        match self {
            ShieldFromAssetLockTransitionAction::V0(transition) => transition.shield_amount,
        }
    }
    /// Get note commitments
    pub fn note_commitments(&self) -> &Vec<[u8; 32]> {
        match self {
            ShieldFromAssetLockTransitionAction::V0(transition) => &transition.note_commitments,
        }
    }
    /// Get encrypted notes
    pub fn encrypted_notes(&self) -> &Vec<Vec<u8>> {
        match self {
            ShieldFromAssetLockTransitionAction::V0(transition) => &transition.encrypted_notes,
        }
    }
    /// fee multiplier
    pub fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            ShieldFromAssetLockTransitionAction::V0(transition) => transition.user_fee_increase,
        }
    }
}
