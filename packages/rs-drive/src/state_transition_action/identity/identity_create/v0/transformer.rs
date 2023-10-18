use crate::state_transition_action::identity::identity_create::v0::IdentityCreateTransitionActionV0;
use dpp::consensus::basic::identity::IdentityAssetLockTransactionOutputNotFoundError;
use dpp::consensus::basic::BasicError;
use dpp::consensus::ConsensusError;

use dpp::state_transition::state_transitions::identity::identity_create_transition::v0::IdentityCreateTransitionV0;

impl IdentityCreateTransitionActionV0 {
    /// try from
    pub fn try_from(
        value: IdentityCreateTransitionV0,
        initial_balance_amount: u64,
    ) -> Result<Self, ConsensusError> {
        let IdentityCreateTransitionV0 {
            public_keys,
            identity_id,
            asset_lock_proof,
            ..
        } = value;
        let asset_lock_outpoint =
            asset_lock_proof
                .out_point()
                .ok_or(ConsensusError::BasicError(
                    BasicError::IdentityAssetLockTransactionOutputNotFoundError(
                        IdentityAssetLockTransactionOutputNotFoundError::new(
                            asset_lock_proof.instant_lock_output_index().unwrap(),
                        ),
                    ),
                ))?;
        Ok(IdentityCreateTransitionActionV0 {
            public_keys: public_keys.into_iter().map(|a| a.into()).collect(),
            initial_balance_amount,
            identity_id,
            asset_lock_outpoint,
        })
    }

    /// try from borrowed
    pub fn try_from_borrowed(
        value: &IdentityCreateTransitionV0,
        initial_balance_amount: u64,
    ) -> Result<Self, ConsensusError> {
        let IdentityCreateTransitionV0 {
            public_keys,
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
        Ok(IdentityCreateTransitionActionV0 {
            public_keys: public_keys.iter().map(|key| key.into()).collect(),
            initial_balance_amount,
            identity_id: *identity_id,
            asset_lock_outpoint,
        })
    }
}
