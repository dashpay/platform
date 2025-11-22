use crate::state_transition_action::identity::identity_create_from_addresses::v0::IdentityCreateFromAddressesTransitionActionV0;
use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
use dpp::consensus::basic::identity::IdentityAssetLockTransactionOutputNotFoundError;
use dpp::consensus::ConsensusError;
use dpp::platform_value::Bytes36;
use dpp::state_transition::signable_bytes_hasher::SignableBytesHasher;

use dpp::state_transition::state_transitions::identity::identity_create_from_addresses_transition::v0::IdentityCreateFromAddressesTransitionV0;

impl IdentityCreateFromAddressesTransitionActionV0 {
    /// try from
    pub fn try_from(
        value: IdentityCreateFromAddressesTransitionV0,
    ) -> Result<Self, ConsensusError> {
        let IdentityCreateFromAddressesTransitionV0 {
            inputs,
            public_keys,
            identity_id,
            user_fee_increase,
            ..
        } = value;

        Ok(IdentityCreateFromAddressesTransitionActionV0 {
            inputs: inputs
                .into_iter()
                .map(|(key, (_, amount))| (key, amount))
                .collect(),
            public_keys: public_keys.into_iter().map(|a| a.into()).collect(),
            identity_id,
            user_fee_increase,
        })
    }

    /// try from borrowed
    pub fn try_from_borrowed(
        value: &IdentityCreateFromAddressesTransitionV0,
    ) -> Result<Self, ConsensusError> {
        let IdentityCreateFromAddressesTransitionV0 {
            inputs,
            public_keys,
            identity_id,
            user_fee_increase,
            ..
        } = value;

        Ok(IdentityCreateFromAddressesTransitionActionV0 {
            inputs: inputs
                .into_iter()
                .map(|(key, (_, amount))| (key, amount))
                .collect(),
            public_keys: public_keys.iter().map(|key| key.into()).collect(),
            identity_id: *identity_id,
            user_fee_increase: *user_fee_increase,
        })
    }
}
