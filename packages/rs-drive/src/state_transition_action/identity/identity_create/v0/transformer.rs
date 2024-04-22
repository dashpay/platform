use crate::state_transition_action::identity::identity_create::v0::IdentityCreateTransitionActionV0;
use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
use dpp::consensus::basic::identity::IdentityAssetLockTransactionOutputNotFoundError;
use dpp::consensus::ConsensusError;
use dpp::platform_value::Bytes36;
use dpp::state_transition::signable_bytes_hasher::SignableBytesHasher;

use dpp::state_transition::state_transitions::identity::identity_create_transition::v0::IdentityCreateTransitionV0;

impl IdentityCreateTransitionActionV0 {
    /// try from
    pub fn try_from(
        value: IdentityCreateTransitionV0,
        signable_bytes_hasher: SignableBytesHasher,
        asset_lock_value_to_be_consumed: AssetLockValue,
    ) -> Result<Self, ConsensusError> {
        let IdentityCreateTransitionV0 {
            public_keys,
            identity_id,
            asset_lock_proof,
            user_fee_increase,
            ..
        } = value;

        let asset_lock_outpoint = asset_lock_proof.out_point().ok_or_else(|| {
            IdentityAssetLockTransactionOutputNotFoundError::new(
                asset_lock_proof.output_index() as usize
            )
        })?;

        Ok(IdentityCreateTransitionActionV0 {
            signable_bytes_hasher,
            public_keys: public_keys.into_iter().map(|a| a.into()).collect(),
            asset_lock_value_to_be_consumed,
            identity_id,
            asset_lock_outpoint: Bytes36::new(asset_lock_outpoint.into()),
            user_fee_increase,
        })
    }

    /// try from borrowed
    pub fn try_from_borrowed(
        value: &IdentityCreateTransitionV0,
        signable_bytes_hasher: SignableBytesHasher,
        asset_lock_value_to_be_consumed: AssetLockValue,
    ) -> Result<Self, ConsensusError> {
        let IdentityCreateTransitionV0 {
            public_keys,
            identity_id,
            asset_lock_proof,
            user_fee_increase,
            ..
        } = value;

        // This should already be checked in validate basic
        let asset_lock_outpoint = asset_lock_proof.out_point().ok_or_else(|| {
            IdentityAssetLockTransactionOutputNotFoundError::new(
                asset_lock_proof.output_index() as usize
            )
        })?;

        Ok(IdentityCreateTransitionActionV0 {
            signable_bytes_hasher,
            public_keys: public_keys.iter().map(|key| key.into()).collect(),
            asset_lock_value_to_be_consumed,
            identity_id: *identity_id,
            asset_lock_outpoint: Bytes36::new(asset_lock_outpoint.into()),
            user_fee_increase: *user_fee_increase,
        })
    }
}
