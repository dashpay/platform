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

    /// Get outputs (Some = explicit amount, None = remainder recipient)
    pub fn outputs(&self) -> &BTreeMap<PlatformAddress, Option<Credits>> {
        match self {
            AddressFundingFromAssetLockTransitionAction::V0(transition) => &transition.outputs,
        }
    }

    /// Get asset lock value to be consumed
    pub fn asset_lock_value_to_be_consumed(&self) -> &AssetLockValue {
        match self {
            AddressFundingFromAssetLockTransitionAction::V0(transition) => {
                &transition.asset_lock_value_to_be_consumed
            }
        }
    }

    /// Get resolved outputs with remainder computed.
    /// Returns outputs where all Option<Credits> are resolved to concrete Credits values.
    pub fn resolved_outputs(&self) -> BTreeMap<PlatformAddress, Credits> {
        use dpp::asset_lock::reduced_asset_lock_value::AssetLockValueGettersV0;

        let outputs = self.outputs();
        let asset_lock_balance = self
            .asset_lock_value_to_be_consumed()
            .remaining_credit_value();

        // Calculate the sum of explicit outputs
        let explicit_outputs_sum: Credits = outputs.values().flatten().sum();

        // Calculate remainder
        let remainder_balance = asset_lock_balance.saturating_sub(explicit_outputs_sum);

        // Resolve all outputs
        outputs
            .iter()
            .map(|(address, balance_option)| {
                let balance = match balance_option {
                    Some(explicit_amount) => *explicit_amount,
                    None => remainder_balance,
                };
                (*address, balance)
            })
            .collect()
    }

    /// Returns owned copies of inputs and outputs.
    #[allow(clippy::type_complexity)]
    pub fn inputs_with_remaining_balance_outputs_and_asset_lock_value_owned(
        self,
    ) -> (
        BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        BTreeMap<PlatformAddress, Option<Credits>>,
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

    /// Removes the remainder output (the one with None value) from the action.
    /// This should be called when total available funds exactly match explicit outputs.
    pub fn remove_remainder_output(&mut self) {
        match self {
            AddressFundingFromAssetLockTransitionAction::V0(transition) => {
                transition.outputs.retain(|_, v| v.is_some());
            }
        }
    }
}
