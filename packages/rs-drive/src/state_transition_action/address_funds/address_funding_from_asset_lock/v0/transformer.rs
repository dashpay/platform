use crate::state_transition_action::address_funds::address_funding_from_asset_lock::v0::AddressFundingFromAssetLockTransitionActionV0;
use dpp::address_funds::PlatformAddress;
use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
use dpp::consensus::basic::identity::IdentityAssetLockTransactionOutputNotFoundError;
use dpp::consensus::ConsensusError;
use dpp::fee::Credits;
use dpp::platform_value::Bytes36;
use dpp::prelude::AddressNonce;
use dpp::state_transition::signable_bytes_hasher::SignableBytesHasher;
use dpp::state_transition::state_transitions::address_funds::address_funding_from_asset_lock_transition::v0::AddressFundingFromAssetLockTransitionV0;
use std::collections::BTreeMap;

impl AddressFundingFromAssetLockTransitionActionV0 {
    /// Transforms the state transition into an action using pre-validated asset lock and input balances.
    ///
    /// # Arguments
    /// * `value` - The state transition
    /// * `signable_bytes_hasher` - The signable bytes hasher from validation
    /// * `asset_lock_value_to_be_consumed` - The asset lock value from validation
    /// * `inputs_with_remaining_balance` - Pre-validated inputs with remaining balances
    pub fn try_from_transition(
        value: &AddressFundingFromAssetLockTransitionV0,
        signable_bytes_hasher: SignableBytesHasher,
        asset_lock_value_to_be_consumed: AssetLockValue,
        inputs_with_remaining_balance: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    ) -> Result<Self, ConsensusError> {
        let AddressFundingFromAssetLockTransitionV0 {
            asset_lock_proof,
            outputs,
            fee_strategy,
            user_fee_increase,
            ..
        } = value;

        let asset_lock_outpoint = asset_lock_proof.out_point().ok_or_else(|| {
            IdentityAssetLockTransactionOutputNotFoundError::new(
                asset_lock_proof.output_index() as usize
            )
        })?;

        Ok(AddressFundingFromAssetLockTransitionActionV0 {
            signable_bytes_hasher,
            asset_lock_value_to_be_consumed,
            asset_lock_outpoint: Bytes36::new(asset_lock_outpoint.into()),
            inputs_with_remaining_balance,
            outputs: outputs.clone(),
            fee_strategy: fee_strategy.clone(),
            user_fee_increase: *user_fee_increase,
        })
    }
}
