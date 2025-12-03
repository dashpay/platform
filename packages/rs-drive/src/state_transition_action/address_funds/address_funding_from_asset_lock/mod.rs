/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::address_funds::address_funding_from_asset_lock::v0::AddressFundingFromAssetLockTransitionActionV0;
use derive_more::From;
use dpp::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
use dpp::fee::Credits;
use dpp::platform_value::Bytes36;
use dpp::prelude::{AddressNonce, UserFeeIncrease};
use std::collections::BTreeMap;

/// action
#[derive(Debug, Clone, From)]
pub enum AddressFundingFromAssetLockTransitionAction {
    /// v0
    V0(AddressFundingFromAssetLockTransitionActionV0),
}

impl AddressFundingFromAssetLockTransitionAction {
    /// Get inputs with remaining balance
    pub fn inputs_with_remaining_balance(
        &self,
    ) -> &BTreeMap<PlatformAddress, (AddressNonce, Credits)> {
        match self {
            AddressFundingFromAssetLockTransitionAction::V0(transition) => {
                &transition.inputs_with_remaining_balance
            }
        }
    }

    /// Get outputs
    pub fn outputs(&self) -> &BTreeMap<PlatformAddress, Credits> {
        match self {
            AddressFundingFromAssetLockTransitionAction::V0(transition) => &transition.outputs,
        }
    }

    /// Returns owned copies of inputs and outputs.
    pub fn inputs_with_remaining_balance_outputs_and_asset_lock_value_owned(
        self,
    ) -> (
        BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        BTreeMap<PlatformAddress, Credits>,
        AssetLockValue,
    ) {
        match self {
            AddressFundingFromAssetLockTransitionAction::V0(transition) => (
                transition.inputs_with_remaining_balance,
                transition.outputs,
                transition.asset_lock_value_to_be_consumed,
            ),
        }
    }

    /// Asset Lock Outpoint
    pub fn asset_lock_outpoint(&self) -> Bytes36 {
        match self {
            AddressFundingFromAssetLockTransitionAction::V0(action) => action.asset_lock_outpoint,
        }
    }

    /// fee multiplier
    pub fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            AddressFundingFromAssetLockTransitionAction::V0(transition) => {
                transition.user_fee_increase
            }
        }
    }

    /// fee strategy
    pub fn fee_strategy(&self) -> &AddressFundsFeeStrategy {
        match self {
            AddressFundingFromAssetLockTransitionAction::V0(transition) => &transition.fee_strategy,
        }
    }
}
