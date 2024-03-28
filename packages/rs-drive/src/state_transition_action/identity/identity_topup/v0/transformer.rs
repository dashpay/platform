use crate::state_transition_action::identity::identity_topup::v0::IdentityTopUpTransitionActionV0;
use dpp::consensus::basic::identity::IdentityAssetLockTransactionOutputNotFoundError;

use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
use dpp::consensus::ConsensusError;
use dpp::platform_value::Bytes36;
use dpp::state_transition::state_transitions::identity::identity_topup_transition::v0::IdentityTopUpTransitionV0;

impl IdentityTopUpTransitionActionV0 {
    /// try from
    pub fn try_from(
        value: IdentityTopUpTransitionV0,
        top_up_asset_lock_value: AssetLockValue,
    ) -> Result<Self, ConsensusError> {
        let IdentityTopUpTransitionV0 {
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

        Ok(IdentityTopUpTransitionActionV0 {
            top_up_asset_lock_value,
            identity_id,
            asset_lock_outpoint: Bytes36::new(asset_lock_outpoint.into()),
            user_fee_increase,
        })
    }

    /// try from borrowed
    pub fn try_from_borrowed(
        value: &IdentityTopUpTransitionV0,
        top_up_asset_lock_value: AssetLockValue,
    ) -> Result<Self, ConsensusError> {
        let IdentityTopUpTransitionV0 {
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

        Ok(IdentityTopUpTransitionActionV0 {
            top_up_asset_lock_value,
            identity_id: *identity_id,
            asset_lock_outpoint: Bytes36::new(asset_lock_outpoint.into()),
            user_fee_increase: *user_fee_increase,
        })
    }
}
