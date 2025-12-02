use crate::state_transition_action::address_funds::address_funding_from_asset_lock::v0::AddressFundingFromAssetLockTransitionActionV0;
use crate::state_transition_action::address_funds::address_funding_from_asset_lock::AddressFundingFromAssetLockTransitionAction;
use dpp::address_funds::PlatformAddress;
use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
use dpp::consensus::ConsensusError;
use dpp::fee::Credits;
use dpp::prelude::AddressNonce;
use dpp::state_transition::address_funding_from_asset_lock_transition::AddressFundingFromAssetLockTransition;
use dpp::state_transition::signable_bytes_hasher::SignableBytesHasher;
use std::collections::BTreeMap;

impl AddressFundingFromAssetLockTransitionAction {
    /// Transforms the state transition into an action using pre-validated asset lock and input balances.
    ///
    /// # Arguments
    /// * `value` - The state transition
    /// * `signable_bytes_hasher` - The signable bytes hasher from validation
    /// * `asset_lock_value_to_be_consumed` - The asset lock value from validation
    /// * `inputs_with_remaining_balance` - Pre-validated inputs with remaining balances
    pub fn try_from_transition(
        value: &AddressFundingFromAssetLockTransition,
        signable_bytes_hasher: SignableBytesHasher,
        asset_lock_value_to_be_consumed: AssetLockValue,
        inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    ) -> Result<Self, ConsensusError> {
        match value {
            AddressFundingFromAssetLockTransition::V0(v0) => Ok(
                AddressFundingFromAssetLockTransitionActionV0::try_from_transition(
                    v0,
                    signable_bytes_hasher,
                    asset_lock_value_to_be_consumed,
                    inputs_with_remaining_balance,
                )?
                .into(),
            ),
        }
    }
}
