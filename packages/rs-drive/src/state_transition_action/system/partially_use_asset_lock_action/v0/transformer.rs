use crate::state_transition_action::identity::identity_create::v0::IdentityCreateTransitionActionV0;
use crate::state_transition_action::identity::identity_topup::v0::IdentityTopUpTransitionActionV0;
use crate::state_transition_action::system::partially_use_asset_lock_action::v0::PartiallyUseAssetLockActionV0;
use dpp::asset_lock::reduced_asset_lock_value::AssetLockValueGettersV0;
use dpp::consensus::basic::identity::IdentityAssetLockTransactionOutputNotFoundError;
use dpp::consensus::ConsensusError;
use dpp::fee::Credits;
use dpp::platform_value::{Bytes32, Bytes36};
use dpp::state_transition::identity_create_transition::v0::IdentityCreateTransitionV0;
use dpp::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;
use dpp::util::hash::hash_double;

impl PartiallyUseAssetLockActionV0 {
    /// try from identity create transition
    pub fn try_from_identity_create_transition(
        value: IdentityCreateTransitionV0,
        signable_bytes: Vec<u8>,
        asset_lock_initial_balance_amount: Credits,
        asset_lock_output_script: Vec<u8>,
        asset_lock_remaining_balance_amount: Credits,
        mut previous_transaction_hashes: Vec<Bytes32>,
        desired_used_credits: Credits,
    ) -> Result<Self, ConsensusError> {
        let signable_bytes_hash = hash_double(signable_bytes);
        previous_transaction_hashes.push(signable_bytes_hash.into());

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

        let remaining_balance_after_used_credits_are_deducted =
            asset_lock_remaining_balance_amount.saturating_sub(desired_used_credits);

        // In the case that we want to pay for processing on a partially used asset lock, and we have already done that
        // processing, and also that the processing was more than the balance on the asset lock it's better just to take
        // the remaining balance.
        let used_credits = std::cmp::min(asset_lock_remaining_balance_amount, desired_used_credits);

        Ok(PartiallyUseAssetLockActionV0 {
            asset_lock_outpoint: Bytes36::new(asset_lock_outpoint.into()),
            initial_credit_value: asset_lock_initial_balance_amount,
            previous_transaction_hashes,
            asset_lock_script: asset_lock_output_script,
            remaining_credit_value: remaining_balance_after_used_credits_are_deducted,
            used_credits,
            user_fee_increase,
        })
    }

    /// try from borrowed identity create transition
    pub fn try_from_borrowed_identity_create_transition(
        value: &IdentityCreateTransitionV0,
        signable_bytes: Vec<u8>,
        asset_lock_initial_balance_amount: Credits,
        asset_lock_output_script: Vec<u8>,
        asset_lock_remaining_balance_amount: Credits,
        mut previous_transaction_hashes: Vec<Bytes32>,
        desired_used_credits: Credits,
    ) -> Result<Self, ConsensusError> {
        let signable_bytes_hash = hash_double(signable_bytes);
        previous_transaction_hashes.push(signable_bytes_hash.into());

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

        // Saturating because we can only have 0 left
        let remaining_balance_after_used_credits_are_deducted =
            asset_lock_remaining_balance_amount.saturating_sub(desired_used_credits);

        // In the case that we want to pay for processing on a partially used asset lock, and we have already done that
        // processing, and also that the processing was more than the balance on the asset lock it's better just to take
        // the remaining balance.
        let used_credits = std::cmp::min(asset_lock_remaining_balance_amount, desired_used_credits);

        Ok(PartiallyUseAssetLockActionV0 {
            asset_lock_outpoint: Bytes36::new(asset_lock_outpoint.into()),
            initial_credit_value: asset_lock_initial_balance_amount,
            previous_transaction_hashes,
            asset_lock_script: asset_lock_output_script,
            remaining_credit_value: remaining_balance_after_used_credits_are_deducted,
            used_credits,
            user_fee_increase: *user_fee_increase,
        })
    }

    /// from identity create transition action
    pub fn from_identity_create_transition_action(
        value: IdentityCreateTransitionActionV0,
        desired_used_credits: Credits,
    ) -> Self {
        let IdentityCreateTransitionActionV0 {
            signable_bytes_hasher,
            asset_lock_outpoint,
            asset_lock_value_to_be_consumed,
            user_fee_increase,
            ..
        } = value;

        let remaining_balance_after_used_credits_are_deducted = asset_lock_value_to_be_consumed
            .remaining_credit_value()
            .saturating_sub(desired_used_credits);

        let used_credits = std::cmp::min(
            asset_lock_value_to_be_consumed.remaining_credit_value(),
            desired_used_credits,
        );

        //todo: remove clone
        let mut used_tags = asset_lock_value_to_be_consumed.used_tags_ref().clone();

        used_tags.push(signable_bytes_hasher.into_hashed_bytes());

        PartiallyUseAssetLockActionV0 {
            asset_lock_outpoint,
            initial_credit_value: asset_lock_value_to_be_consumed.initial_credit_value(),
            previous_transaction_hashes: used_tags,
            asset_lock_script: asset_lock_value_to_be_consumed.tx_out_script_owned(),
            remaining_credit_value: remaining_balance_after_used_credits_are_deducted,
            used_credits,
            user_fee_increase,
        }
    }

    /// from borrowed identity create transition action
    pub fn from_borrowed_identity_create_transition_action(
        value: &IdentityCreateTransitionActionV0,
        desired_used_credits: Credits,
    ) -> Self {
        let IdentityCreateTransitionActionV0 {
            signable_bytes_hasher,
            asset_lock_outpoint,
            asset_lock_value_to_be_consumed,
            user_fee_increase,
            ..
        } = value;

        let remaining_balance_after_used_credits_are_deducted = asset_lock_value_to_be_consumed
            .remaining_credit_value()
            .saturating_sub(desired_used_credits);

        let used_credits = std::cmp::min(
            asset_lock_value_to_be_consumed.remaining_credit_value(),
            desired_used_credits,
        );

        let mut used_tags = asset_lock_value_to_be_consumed.used_tags_ref().clone();

        used_tags.push(signable_bytes_hasher.to_hashed_bytes());

        PartiallyUseAssetLockActionV0 {
            asset_lock_outpoint: *asset_lock_outpoint,
            initial_credit_value: asset_lock_value_to_be_consumed.initial_credit_value(),
            previous_transaction_hashes: used_tags,
            asset_lock_script: asset_lock_value_to_be_consumed.tx_out_script().clone(),
            remaining_credit_value: remaining_balance_after_used_credits_are_deducted,
            used_credits,
            user_fee_increase: *user_fee_increase,
        }
    }

    /// try from identity top up transition
    pub fn try_from_identity_top_up_transition(
        value: IdentityTopUpTransitionV0,
        signable_bytes: Vec<u8>,
        asset_lock_initial_balance_amount: Credits,
        asset_lock_output_script: Vec<u8>,
        asset_lock_remaining_balance_amount: Credits,
        mut previous_transaction_hashes: Vec<Bytes32>,
        desired_used_credits: Credits,
    ) -> Result<Self, ConsensusError> {
        let signable_bytes_hash = hash_double(signable_bytes);
        previous_transaction_hashes.push(signable_bytes_hash.into());

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

        let remaining_balance_after_used_credits_are_deducted =
            asset_lock_remaining_balance_amount.saturating_sub(desired_used_credits);

        // In the case that we want to pay for processing on a partially used asset lock, and we have already done that
        // processing, and also that the processing was more than the balance on the asset lock it's better just to take
        // the remaining balance.
        let used_credits = std::cmp::min(asset_lock_remaining_balance_amount, desired_used_credits);

        Ok(PartiallyUseAssetLockActionV0 {
            asset_lock_outpoint: Bytes36::new(asset_lock_outpoint.into()),
            initial_credit_value: asset_lock_initial_balance_amount,
            previous_transaction_hashes,
            asset_lock_script: asset_lock_output_script,
            remaining_credit_value: remaining_balance_after_used_credits_are_deducted,
            used_credits,
            user_fee_increase,
        })
    }

    /// try from borrowed identity top up transition
    pub fn try_from_borrowed_identity_top_up_transition(
        value: &IdentityTopUpTransitionV0,
        signable_bytes: Vec<u8>,
        asset_lock_initial_balance_amount: Credits,
        asset_lock_output_script: Vec<u8>,
        asset_lock_remaining_balance_amount: Credits,
        mut previous_transaction_hashes: Vec<Bytes32>,
        desired_used_credits: Credits,
    ) -> Result<Self, ConsensusError> {
        let signable_bytes_hash = hash_double(signable_bytes);
        previous_transaction_hashes.push(signable_bytes_hash.into());

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

        let remaining_balance_after_used_credits_are_deducted =
            asset_lock_remaining_balance_amount.saturating_sub(desired_used_credits);

        // In the case that we want to pay for processing on a partially used asset lock, and we have already done that
        // processing, and also that the processing was more than the balance on the asset lock it's better just to take
        // the remaining balance.
        let used_credits = std::cmp::min(asset_lock_remaining_balance_amount, desired_used_credits);

        Ok(PartiallyUseAssetLockActionV0 {
            asset_lock_outpoint: Bytes36::new(asset_lock_outpoint.into()),
            initial_credit_value: asset_lock_initial_balance_amount,
            previous_transaction_hashes,
            asset_lock_script: asset_lock_output_script,
            remaining_credit_value: remaining_balance_after_used_credits_are_deducted,
            used_credits,
            user_fee_increase: *user_fee_increase,
        })
    }

    /// from identity top up transition action
    pub fn from_identity_top_up_transition_action(
        value: IdentityTopUpTransitionActionV0,
        desired_used_credits: Credits,
    ) -> Self {
        let IdentityTopUpTransitionActionV0 {
            signable_bytes_hasher,
            asset_lock_outpoint,
            top_up_asset_lock_value,
            user_fee_increase,
            ..
        } = value;

        let remaining_balance_after_used_credits_are_deducted = top_up_asset_lock_value
            .remaining_credit_value()
            .saturating_sub(desired_used_credits);

        let used_credits = std::cmp::min(
            top_up_asset_lock_value.remaining_credit_value(),
            desired_used_credits,
        );

        let mut used_tags = top_up_asset_lock_value.used_tags_ref().clone();

        used_tags.push(signable_bytes_hasher.into_hashed_bytes());

        PartiallyUseAssetLockActionV0 {
            asset_lock_outpoint,
            initial_credit_value: top_up_asset_lock_value.initial_credit_value(),
            previous_transaction_hashes: used_tags,
            asset_lock_script: top_up_asset_lock_value.tx_out_script_owned(),
            remaining_credit_value: remaining_balance_after_used_credits_are_deducted,
            used_credits,
            user_fee_increase,
        }
    }

    /// from borrowed identity top up transition
    pub fn from_borrowed_identity_top_up_transition_action(
        value: &IdentityTopUpTransitionActionV0,
        desired_used_credits: Credits,
    ) -> Self {
        let IdentityTopUpTransitionActionV0 {
            signable_bytes_hasher,
            asset_lock_outpoint,
            top_up_asset_lock_value,
            user_fee_increase,
            ..
        } = value;

        let remaining_balance_after_used_credits_are_deducted = top_up_asset_lock_value
            .remaining_credit_value()
            .saturating_sub(desired_used_credits);

        let used_credits = std::cmp::min(
            top_up_asset_lock_value.remaining_credit_value(),
            desired_used_credits,
        );

        let mut used_tags = top_up_asset_lock_value.used_tags_ref().clone();

        used_tags.push(signable_bytes_hasher.to_hashed_bytes());

        PartiallyUseAssetLockActionV0 {
            asset_lock_outpoint: *asset_lock_outpoint,
            initial_credit_value: top_up_asset_lock_value.initial_credit_value(),
            previous_transaction_hashes: used_tags,
            asset_lock_script: top_up_asset_lock_value.tx_out_script().clone(),
            remaining_credit_value: remaining_balance_after_used_credits_are_deducted,
            used_credits,
            user_fee_increase: *user_fee_increase,
        }
    }
}
