use crate::state_transition_action::system::partially_use_asset_lock_action::v0::PartiallyUseAssetLockActionV0;
use dpp::consensus::basic::decode::SerializedObjectParsingError;
use dpp::consensus::basic::identity::IdentityAssetLockTransactionOutputNotFoundError;
use dpp::consensus::ConsensusError;
use dpp::fee::Credits;
use dpp::platform_value::Bytes36;
use dpp::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
use dpp::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;
use std::io;

impl PartiallyUseAssetLockActionV0 {
    /// try from identity create transition
    pub fn try_from_identity_create_transition(
        value: IdentityCreateTransitionV0,
        asset_lock_initial_balance_amount: Credits,
        asset_lock_remaining_balance_amount: Credits,
        desired_used_credits: Credits,
    ) -> Result<Self, ConsensusError> {
        let IdentityCreateTransitionV0 {
            asset_lock_proof,
            user_fee_increase,
            ..
        } = value;

        let asset_lock_outpoint = asset_lock_proof.out_point().ok_or_else(|| {
            IdentityAssetLockTransactionOutputNotFoundError::new(
                asset_lock_proof.output_index() as usize
            )
        })?;

        let outpoint_bytes = asset_lock_outpoint
            .try_into()
            .map_err(|e: io::Error| SerializedObjectParsingError::new(e.to_string()))?;

        let remaining_balance_after_used_credits_are_deducted =
            asset_lock_remaining_balance_amount.saturating_sub(desired_used_credits);

        let used_credits = std::cmp::min(asset_lock_remaining_balance_amount, desired_used_credits);

        Ok(PartiallyUseAssetLockActionV0 {
            asset_lock_outpoint: Bytes36::new(outpoint_bytes),
            initial_credit_value: asset_lock_initial_balance_amount,
            remaining_credit_value: remaining_balance_after_used_credits_are_deducted,
            used_credits,
            user_fee_increase,
        })
    }

    /// try from borrowed identity create transition
    pub fn try_from_borrowed_identity_create_transition(
        value: &IdentityCreateTransitionV0,
        asset_lock_initial_balance_amount: Credits,
        asset_lock_remaining_balance_amount: Credits,
        desired_used_credits: Credits,
    ) -> Result<Self, ConsensusError> {
        let IdentityCreateTransitionV0 {
            asset_lock_proof,
            user_fee_increase,
            ..
        } = value;

        let asset_lock_outpoint = asset_lock_proof.out_point().ok_or_else(|| {
            IdentityAssetLockTransactionOutputNotFoundError::new(
                asset_lock_proof.output_index() as usize
            )
        })?;

        let outpoint_bytes = asset_lock_outpoint
            .try_into()
            .map_err(|e: io::Error| SerializedObjectParsingError::new(e.to_string()))?;

        // Saturating because we can only have 0 left
        let remaining_balance_after_used_credits_are_deducted =
            asset_lock_remaining_balance_amount.saturating_sub(desired_used_credits);

        // we can only charge what is left, since the operation already happened, we just take what is left.
        let used_credits = std::cmp::min(asset_lock_remaining_balance_amount, desired_used_credits);

        Ok(PartiallyUseAssetLockActionV0 {
            asset_lock_outpoint: Bytes36::new(outpoint_bytes),
            initial_credit_value: asset_lock_initial_balance_amount,
            remaining_credit_value: remaining_balance_after_used_credits_are_deducted,
            used_credits,
            user_fee_increase: *user_fee_increase,
        })
    }

    /// try from identity top up transition
    pub fn try_from_identity_top_up_transition(
        value: IdentityTopUpTransitionV0,
        asset_lock_initial_balance_amount: Credits,
        asset_lock_remaining_balance_amount: Credits,
        desired_used_credits: Credits,
    ) -> Result<Self, ConsensusError> {
        let IdentityTopUpTransitionV0 {
            asset_lock_proof,
            user_fee_increase,
            ..
        } = value;

        let asset_lock_outpoint = asset_lock_proof.out_point().ok_or_else(|| {
            IdentityAssetLockTransactionOutputNotFoundError::new(
                asset_lock_proof.output_index() as usize
            )
        })?;

        let outpoint_bytes = asset_lock_outpoint
            .try_into()
            .map_err(|e: io::Error| SerializedObjectParsingError::new(e.to_string()))?;

        let remaining_balance_after_used_credits_are_deducted =
            asset_lock_remaining_balance_amount.saturating_sub(desired_used_credits);

        let used_credits = std::cmp::min(asset_lock_remaining_balance_amount, desired_used_credits);

        Ok(PartiallyUseAssetLockActionV0 {
            asset_lock_outpoint: Bytes36::new(outpoint_bytes),
            initial_credit_value: asset_lock_initial_balance_amount,
            remaining_credit_value: remaining_balance_after_used_credits_are_deducted,
            used_credits,
            user_fee_increase,
        })
    }

    /// try from borrowed identity top up transition
    pub fn try_from_borrowed_identity_top_up_transition(
        value: &IdentityTopUpTransitionV0,
        asset_lock_initial_balance_amount: Credits,
        asset_lock_remaining_balance_amount: Credits,
        desired_used_credits: Credits,
    ) -> Result<Self, ConsensusError> {
        let IdentityTopUpTransitionV0 {
            asset_lock_proof,
            user_fee_increase,
            ..
        } = value;

        let asset_lock_outpoint = asset_lock_proof.out_point().ok_or_else(|| {
            IdentityAssetLockTransactionOutputNotFoundError::new(
                asset_lock_proof.output_index() as usize
            )
        })?;

        let outpoint_bytes = asset_lock_outpoint
            .try_into()
            .map_err(|e: io::Error| SerializedObjectParsingError::new(e.to_string()))?;

        let remaining_balance_after_used_credits_are_deducted =
            asset_lock_remaining_balance_amount.saturating_sub(desired_used_credits);

        let used_credits = std::cmp::min(asset_lock_remaining_balance_amount, desired_used_credits);

        Ok(PartiallyUseAssetLockActionV0 {
            asset_lock_outpoint: Bytes36::new(outpoint_bytes),
            initial_credit_value: asset_lock_initial_balance_amount,
            remaining_credit_value: remaining_balance_after_used_credits_are_deducted,
            used_credits,
            user_fee_increase: *user_fee_increase,
        })
    }
}
