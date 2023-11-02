use crate::state_transition_action::identity::identity_topup::v0::IdentityTopUpTransitionActionV0;
use dpp::consensus::basic::decode::SerializedObjectParsingError;
use dpp::consensus::basic::identity::IdentityAssetLockTransactionOutputNotFoundError;
use dpp::consensus::basic::BasicError;
use dpp::consensus::ConsensusError;
use dpp::fee::Credits;
use dpp::platform_value::Bytes36;
use dpp::state_transition::state_transitions::identity::identity_topup_transition::v0::IdentityTopUpTransitionV0;
use std::io;

impl IdentityTopUpTransitionActionV0 {
    /// try from
    pub fn try_from(
        value: IdentityTopUpTransitionV0,
        top_up_balance_amount: Credits,
    ) -> Result<Self, ConsensusError> {
        let IdentityTopUpTransitionV0 {
            identity_id,
            asset_lock_proof,
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

        Ok(IdentityTopUpTransitionActionV0 {
            top_up_balance_amount,
            identity_id,
            asset_lock_outpoint: Bytes36::new(outpoint_bytes),
        })
    }

    /// try from borrowed
    pub fn try_from_borrowed(
        value: &IdentityTopUpTransitionV0,
        top_up_balance_amount: Credits,
    ) -> Result<Self, ConsensusError> {
        let IdentityTopUpTransitionV0 {
            identity_id,
            asset_lock_proof,
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

        Ok(IdentityTopUpTransitionActionV0 {
            top_up_balance_amount,
            identity_id: *identity_id,
            asset_lock_outpoint: Bytes36::new(outpoint_bytes),
        })
    }
}
