use crate::state_transition_action::identity::identity_topup_from_addresses::v0::IdentityTopUpFromAddressesTransitionActionV0;
use dpp::consensus::basic::identity::IdentityAssetLockTransactionOutputNotFoundError;

use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
use dpp::consensus::ConsensusError;
use dpp::platform_value::Bytes36;
use dpp::state_transition::signable_bytes_hasher::SignableBytesHasher;
use dpp::state_transition::state_transitions::identity::identity_topup_from_addresses_transition::v0::IdentityTopUpFromAddressesTransitionV0;

impl IdentityTopUpFromAddressesTransitionActionV0 {
    /// try from
    pub fn try_from(value: IdentityTopUpFromAddressesTransitionV0) -> Result<Self, ConsensusError> {
        let IdentityTopUpFromAddressesTransitionV0 {
            identity_id,
            inputs,
            user_fee_increase,
            ..
        } = value;

        Ok(IdentityTopUpFromAddressesTransitionActionV0 {
            inputs,
            identity_id,
            user_fee_increase,
        })
    }

    /// try from borrowed
    pub fn try_from_borrowed(
        value: &IdentityTopUpFromAddressesTransitionV0,
    ) -> Result<Self, ConsensusError> {
        let IdentityTopUpFromAddressesTransitionV0 {
            identity_id,
            inputs,
            user_fee_increase,
            ..
        } = value;

        Ok(IdentityTopUpFromAddressesTransitionActionV0 {
            inputs: inputs.clone(),
            identity_id: *identity_id,
            user_fee_increase: *user_fee_increase,
        })
    }
}
