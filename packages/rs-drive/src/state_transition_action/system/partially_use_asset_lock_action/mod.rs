use crate::state_transition_action::system::partially_use_asset_lock_action::v0::PartiallyUseAssetLockActionV0;
use derive_more::From;
use dpp::fee::Credits;
use dpp::platform_value::{Bytes32, Bytes36};
use dpp::prelude::UserFeeIncrease;

mod transformer;
mod v0;

pub use v0::PartiallyUseAssetLockActionAccessorsV0;

#[derive(Debug, Clone, From)]
/// An action expressing that an asset lock should be partially used
pub enum PartiallyUseAssetLockAction {
    /// v0
    V0(PartiallyUseAssetLockActionV0),
}

impl PartiallyUseAssetLockActionAccessorsV0 for PartiallyUseAssetLockAction {
    fn asset_lock_outpoint(&self) -> Bytes36 {
        match self {
            PartiallyUseAssetLockAction::V0(transition) => transition.asset_lock_outpoint,
        }
    }

    fn initial_credit_value(&self) -> Credits {
        match self {
            PartiallyUseAssetLockAction::V0(transition) => transition.initial_credit_value,
        }
    }

    fn asset_lock_script(&self) -> &Vec<u8> {
        match self {
            PartiallyUseAssetLockAction::V0(transition) => &transition.asset_lock_script,
        }
    }

    fn asset_lock_script_owned(self) -> Vec<u8> {
        match self {
            PartiallyUseAssetLockAction::V0(transition) => transition.asset_lock_script,
        }
    }

    fn remaining_credit_value(&self) -> Credits {
        match self {
            PartiallyUseAssetLockAction::V0(transition) => transition.remaining_credit_value,
        }
    }

    fn used_credits(&self) -> Credits {
        match self {
            PartiallyUseAssetLockAction::V0(transition) => transition.used_credits,
        }
    }

    fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            PartiallyUseAssetLockAction::V0(transition) => transition.user_fee_increase,
        }
    }

    fn previous_transaction_hashes_ref(&self) -> &Vec<Bytes32> {
        match self {
            PartiallyUseAssetLockAction::V0(transition) => &transition.previous_transaction_hashes,
        }
    }
}
