use crate::state_transition_action::identity::identity_topup::v0::IdentityTopUpTransitionActionV0;
use dpp::consensus::basic::identity::IdentityAssetLockTransactionOutputNotFoundError;
use dpp::consensus::basic::BasicError;
use dpp::consensus::ConsensusError;
use dpp::state_transition::state_transitions::identity::identity_topup_transition::v0::IdentityTopUpTransitionV0;

impl IdentityTopUpTransitionActionV0 {
    pub fn try_from(
        value: IdentityTopUpTransitionV0,
        top_up_balance_amount: u64,
    ) -> Result<Self, ConsensusError> {
        let IdentityTopUpTransitionV0 {
            identity_id,
            asset_lock_proof,
            ..
        } = value;
        let asset_lock_outpoint = asset_lock_proof
            .out_point()
            .ok_or(ConsensusError::BasicError(
                BasicError::IdentityAssetLockTransactionOutputNotFoundError(
                    IdentityAssetLockTransactionOutputNotFoundError::new(
                        asset_lock_proof.instant_lock_output_index().unwrap(),
                    ),
                ),
            ))?
            .into();
        Ok(IdentityTopUpTransitionActionV0 {
            top_up_balance_amount,
            identity_id,
            asset_lock_outpoint,
        })
    }

    pub fn try_from_borrowed(
        value: &IdentityTopUpTransitionV0,
        top_up_balance_amount: u64,
    ) -> Result<Self, ConsensusError> {
        let IdentityTopUpTransitionV0 {
            identity_id,
            asset_lock_proof,
            ..
        } = value;
        let asset_lock_outpoint = asset_lock_proof
            .out_point()
            .ok_or(ConsensusError::BasicError(
                BasicError::IdentityAssetLockTransactionOutputNotFoundError(
                    IdentityAssetLockTransactionOutputNotFoundError::new(
                        asset_lock_proof.instant_lock_output_index().unwrap(),
                    ),
                ),
            ))?
            .into();

        Ok(IdentityTopUpTransitionActionV0 {
            top_up_balance_amount,
            identity_id: *identity_id,
            asset_lock_outpoint,
        })
    }
}
